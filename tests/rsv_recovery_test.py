"""Opt-in integration regressions using a local Sony PCM RSV; media stays local.

Fixtures use the first four complete groups of the supplied file. Each metadata
packet must correspond to a video frame (the layout of the reported 50 fps clip).
"""
import argparse
from fractions import Fraction
import json
import mmap
from pathlib import Path
import struct
import subprocess
import tempfile


def probe(path):
    return json.loads(subprocess.check_output([
        "ffprobe", "-v", "error", "-show_streams", "-of", "json", str(path)
    ]))["streams"]


def verify_tables(path):
    # Check table coverage, including the final, shortened composition run.
    with path.open("rb") as file:
        size, kind = struct.unpack(">I4s", file.read(8))
        while kind != b"moov":
            file.seek(size - 8, 1)
            size, kind = struct.unpack(">I4s", file.read(8))
        moov = file.read(size - 8)

    def boxes(data):
        pos = 0
        while pos + 8 <= len(data):
            size, kind = struct.unpack_from(">I4s", data, pos)
            assert size >= 8 and pos + size <= len(data)
            yield kind, data[pos + 8:pos + size]
            pos += size

    def descend(data, names):
        for kind, content in boxes(data):
            if kind == names[0]:
                if len(names) == 1:
                    yield content
                else:
                    yield from descend(content, names[1:])

    for stbl in descend(moov, [b"trak", b"mdia", b"minf", b"stbl"]):
        tables = dict(boxes(stbl))
        sample_count = struct.unpack_from(">I", tables[b"stsz"], 8)[0]
        for kind in (b"stts", b"ctts"):
            if kind not in tables:
                continue
            data = tables[kind]
            count = struct.unpack_from(">I", data, 4)[0]
            assert sum(struct.unpack_from(">I", data, 8 + 8 * i)[0]
                       for i in range(count)) == sample_count, kind


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tool", type=Path, required=True)
    parser.add_argument("--donor", type=Path, required=True)
    parser.add_argument("--rsv", type=Path, required=True)
    args = parser.parse_args()
    streams = probe(args.donor)
    video = next(x for x in streams if x["codec_type"] == "video")
    audio = [x for x in streams if x["codec_type"] == "audio"]
    assert video["codec_name"] == "h264" and len(audio) == 1
    assert audio[0]["codec_name"] in ("pcm_s16be", "pcm_s16le")
    fps = Fraction(video["r_frame_rate"])
    rate = int(audio[0]["sample_rate"])
    sample_bytes = int(audio[0]["channels"]) * 2
    prefix, tag = bytes.fromhex("001c0100"), bytes.fromhex("f0010010")

    with args.rsv.open("rb") as file, mmap.mmap(file.fileno(), 0, access=mmap.ACCESS_READ) as raw:
        hits, pos = [], 0
        while len(hits) < 1024:
            pos = raw.find(prefix, pos)
            if pos < 0:
                break
            if raw[pos + 8:pos + 12] == tag:
                hits.append(pos)
            pos += 1
        step = hits[1] - hits[0]
        groups = []
        for hit in hits:
            if not groups or hit - groups[-1][-1] != step:
                groups.append([hit])
            else:
                groups[-1].append(hit)
        assert len(groups) >= 5, "Need at least four complete groups"
        counts = [len(x) for x in groups[:4]]
        assert len(set(counts)) == 1, "Fixture expects a constant metadata cadence"
        frames = counts[0]
        samples = frames * rate / fps
        assert samples.denominator == 1, "Fixture expects an integer PCM block"
        samples = int(samples)
        audio_bytes = samples * sample_bytes
        end = groups[4][0]
        original = bytes(raw[:end])
        misleading = bytearray(original)
        audio_start = groups[1][0] - audio_bytes
        assert audio_bytes > 65540
        # Valid type-1 header plus a fitting length fools length/header-only
        # walkers. Other GOPs and the video/audio time check must reject it.
        misleading[audio_start:audio_start + 6] = bytes.fromhex("000100004180")
        start = groups[3][-1] + step
        cursor, aud_offsets = start, []
        while len(aud_offsets) < 3:
            length = struct.unpack_from(">I", raw, cursor)[0]
            if raw[cursor + 4] & 31 == 9:
                aud_offsets.append(cursor)
            cursor += 4 + length
        partial_frame = bytes(raw[:aud_offsets[2] + 14])
        partial_metadata = bytes(raw[:end + 12])

    with tempfile.TemporaryDirectory(prefix="repair-rsv-integration-") as directory:
        root = Path(directory)
        cases = [
            ("complete-audio-at-eof", original, 4 * frames, 4 * samples),
            ("legal-looking-pcm-nal", misleading, 4 * frames, 4 * samples),
            ("truncated-video-slice", partial_frame, 3 * frames + 2, 3 * samples),
            ("truncated-metadata", partial_metadata, 4 * frames, 4 * samples),
        ]
        for name, data, expected_frames, expected_samples in cases:
            broken, output = root / (name + ".RSV"), root / (name + ".MP4")
            broken.write_bytes(data)
            result = subprocess.run([str(args.tool.resolve()), "-rsv", "-dst", str(output),
                                     str(args.donor.resolve()), str(broken)], capture_output=True)
            assert result.returncode == 0, result.stderr.decode(errors="replace")
            actual = probe(output)
            v = next(x for x in actual if x["codec_type"] == "video")
            a = next(x for x in actual if x["codec_type"] == "audio")
            assert int(v["nb_frames"]) == expected_frames, (name, v["nb_frames"])
            assert int(a["nb_frames"]) == expected_samples, (name, a["nb_frames"])
            verify_tables(output)
            decode = subprocess.run(["ffmpeg", "-v", "error", "-xerror", "-err_detect", "explode",
                                     "-i", str(output), "-map", "0:v:0", "-map", "0:a:0",
                                     "-f", "null", "-"], capture_output=True)
            assert decode.returncode == 0 and not decode.stderr, (name, decode.stderr)
            print("PASS", name, expected_frames, "frames;", expected_samples, "PCM samples")
        for length in (0, 1, 11, 12):
            broken, output = root / "short.RSV", root / "short.MP4"
            broken.write_bytes(original[:length])
            result = subprocess.run([str(args.tool.resolve()), "-rsv", "-dst", str(output),
                                     str(args.donor.resolve()), str(broken)], capture_output=True)
            assert result.returncode == 1 and not output.exists(), length
        print("PASS short RSV inputs fail without producing an output")


if __name__ == "__main__":
    main()
