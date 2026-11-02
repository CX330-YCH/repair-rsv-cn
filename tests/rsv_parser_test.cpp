#include "rsv_parser.h"
#include <cassert>
#include <iostream>

using Bytes = std::vector<unsigned char>;

static void nal(Bytes& data, std::initializer_list<unsigned char> payload) {
	const auto n = payload.size();
	data.insert(data.end(), {0, 0, 0, static_cast<unsigned char>(n)});
	data.insert(data.end(), payload);
}

static void avcFrame(Bytes& data, bool key = false) {
	nal(data, {9, 0x50});
	nal(data, {6, 0x80});
	nal(data, {static_cast<unsigned char>(key ? 0x65 : 0x41), 0x80});
	nal(data, {0x41, 0x80}); // Multiple slices belong to the same access unit.
}

int main() {
	Bytes video;
	avcFrame(video, true);
	const auto frame_size = video.size();
	avcFrame(video);
	auto clean = rsv::scanVideo(video.data(), video.size(), false);
	assert(clean.finish());
	assert(clean.frames.size() == 2 && clean.frames[0].size == frame_size);
	assert(clean.frames[0].key && !clean.frames[1].key);

	// The reported failure: PCM's 0x00010000 length fits, but type 0 is invalid.
	auto pcm = video;
	pcm.resize(video.size() + 92160);
	pcm[video.size() + 1] = 1;
	auto mixed = rsv::scanVideo(pcm.data(), pcm.size(), false);
	assert(mixed.stop == rsv::Stop::invalid && mixed.end == video.size());
	assert(mixed.has_vcl && mixed.frames.size() == 1);
	assert(rsv::audioMatches(92160, 24, 4, 48000, 1000, 50000));
	assert(!rsv::audioMatches(26620, 24, 4, 48000, 1000, 50000));
	assert(!rsv::audioMatches(92161, 24, 4, 48000, 1000, 50000));
	// Accept neighbouring sample counts at fractional frame rates, without drift
	// from repeatedly rounding a calculated GOP duration.
	assert(rsv::audioMatches(192192, 30, 4, 48000, 1001, 30000));
	assert(rsv::audioMatches(192196, 30, 4, 48000, 1001, 30000));

	// Every possible truncation of the second frame must retain the first only
	// after a complete following AUD confirms its boundary.
	for (std::size_t n = 0; n <= video.size(); ++n) {
		auto scan = rsv::scanVideo(video.data(), n, false);
		assert(scan.end <= n);
		if (n >= frame_size + 6) assert(scan.frames.size() == 1);
	}
	auto truncated = video;
	truncated.pop_back();
	auto tail = rsv::scanVideo(truncated.data(), truncated.size(), false);
	assert(tail.stop == rsv::Stop::truncated && !tail.finish());
	assert(tail.frames.size() == 1 && tail.frames[0].size == frame_size);

	Bytes hevc;
	nal(hevc, {0x46, 1, 0x50}); // AUD
	nal(hevc, {0x26, 1, 0x80}); // IDR
	nal(hevc, {0x02, 1, 0x80}); // Another slice
	auto h = rsv::scanVideo(hevc.data(), hevc.size(), true);
	assert(h.finish() && h.frames.size() == 1 && h.frames[0].key);
	hevc[5] = 0; // nuh_temporal_id_plus1 must be nonzero.
	assert(rsv::scanVideo(hevc.data(), hevc.size(), true).stop == rsv::Stop::invalid);
	Bytes forbidden;
	nal(forbidden, {0x89, 0x50});
	assert(rsv::scanVideo(forbidden.data(), forbidden.size(), false).stop == rsv::Stop::invalid);
	Bytes empty_frame;
	nal(empty_frame, {9, 0x50});
	nal(empty_frame, {9, 0x50});
	assert(!rsv::scanVideo(empty_frame.data(), empty_frame.size(), false).finish());

	const Bytes metadata = {0, 0x1c, 1, 0, 0, 0, 0, 0, 0xf0, 1, 0, 0x10};
	for (std::size_t n = 0; n < metadata.size(); ++n)
		assert(!rsv::metadata(metadata.data(), n));
	assert(rsv::metadata(metadata.data(), metadata.size()));
	assert(rsv::nextMetadata(metadata.data(), metadata.size()) == 0);
	std::cout << "RSV parser regressions passed\n";
}
