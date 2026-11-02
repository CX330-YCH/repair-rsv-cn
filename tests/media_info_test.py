"""Compare the bundled metadata reader with FFprobe on supplied local media."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--tool', type=Path, default=Path('bin/untrunc-rsv'))
    parser.add_argument('media', type=Path, nargs='+')
    args = parser.parse_args()
    tool = str(args.tool.resolve())
    for path in args.media:
        before = path.stat()
        result = subprocess.run([tool, '--probe', str(path)], capture_output=True, check=True)
        info = json.loads(result.stdout)
        assert info['sizeBytes'] == before.st_size
        assert path.stat().st_mtime_ns == before.st_mtime_ns
        if info['unindexed']:
            assert not info['readable'] and info['video'] is None and info['audio'] == []
        else:
            reference = json.loads(subprocess.check_output([
                'ffprobe', '-v', 'error', '-show_streams', '-of', 'json', str(path)
            ]))['streams']
            video = next(x for x in reference if x['codec_type'] == 'video')
            actual = info['video']
            for key in ('width', 'height'):
                assert actual[key] == video[key]
            assert actual['codec'] == video['codec_name']
            if 'nb_frames' in video:
                assert actual['frameCount'] == int(video['nb_frames'])
            if 'duration' in video:
                assert abs(actual['durationSeconds'] - float(video['duration'])) < 0.000001
            for actual, reference in zip(info['audio'], [x for x in reference if x['codec_type'] == 'audio'], strict=True):
                assert actual['sampleRate'] == int(reference['sample_rate'])
                assert actual['channels'] == reference['channels']
        print('PASS metadata:', path.name)
    with tempfile.TemporaryDirectory(prefix='repair-rsv-probe-') as directory:
        path = Path(directory) / '无法读取的文件.bin'
        path.write_bytes(b'not a media file')
        info = json.loads(subprocess.check_output([tool, '--probe', str(path)], stderr=subprocess.DEVNULL))
        assert not info['readable'] and not info['unindexed'] and info['video'] is None
        path.unlink()
        assert subprocess.run([tool, '--probe', str(path)], capture_output=True).returncode != 0
        print('PASS invalid and missing file metadata')


if __name__ == '__main__':
    main()
