// Bounded parsing for Sony RSV's length-prefixed AVC/HEVC access units.
#pragma once

#include <algorithm>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <vector>

namespace rsv {

struct Frame {
	std::size_t offset = 0;
	std::size_t size = 0;
	bool key = false;
};

enum class Stop { end, invalid, truncated };

struct Video {
	std::vector<Frame> frames; // A following AUD proves these access units ended.
	Frame pending;
	bool has_vcl = false;
	std::size_t end = 0;
	Stop stop = Stop::end;

	bool finish() {
		if (stop != Stop::end || !has_vcl) return false;
		pending.size = end - pending.offset;
		frames.push_back(pending);
		has_vcl = false;
		return true;
	}
};

inline bool metadata(const unsigned char* p, std::size_t size) {
	return size >= 12 && std::memcmp(p, "\x00\x1c\x01\x00", 4) == 0 &&
	       std::memcmp(p + 8, "\xf0\x01\x00\x10", 4) == 0;
}

inline std::size_t nextMetadata(const unsigned char* p, std::size_t size) {
	for (std::size_t i = 0; i + 12 <= size; ++i)
		if (p[i] == 0 && metadata(p + i, size - i)) return i;
	return size;
}

inline Video scanVideo(const unsigned char* data, std::size_t size, bool hevc) {
	Video result;
	bool started = false;
	while (result.end < size) {
		const std::size_t cursor = result.end;
		const std::size_t remaining = size - cursor;
		if (remaining < 4) { result.stop = Stop::truncated; break; }
		const auto* p = data + cursor;
		const std::uint32_t length = (std::uint32_t(p[0]) << 24) |
			(std::uint32_t(p[1]) << 16) | (std::uint32_t(p[2]) << 8) | p[3];
		const unsigned header_size = hevc ? 2 : 1;
		if (length < header_size) { result.stop = Stop::invalid; break; }
		if (remaining < 4 + header_size) { result.stop = Stop::truncated; break; }
		const unsigned type = hevc ? ((p[4] >> 1) & 63) : (p[4] & 31);
		// Reject forbidden/reserved headers even when a PCM length happens to fit.
		const bool valid_type = hevc ? (type <= 9 || (type >= 16 && type <= 21) ||
			(type >= 32 && type <= 40)) : (type >= 1 && type <= 12);
		if ((p[4] & 128) || !valid_type || (hevc && !(p[5] & 7))) {
			result.stop = Stop::invalid; break;
		}
		if (length > remaining - 4) { result.stop = Stop::truncated; break; }
		const bool aud = type == (hevc ? 35 : 9);
		if (aud) {
			// primary_pic_type followed by rbsp_stop_one_bit and alignment zeros.
			if (length != header_size + 1 || (p[4 + header_size] & 31) != 16) {
				result.stop = Stop::invalid; break;
			}
			if (started) {
				if (!result.has_vcl) { result.stop = Stop::invalid; break; }
				result.pending.size = cursor - result.pending.offset;
				result.frames.push_back(result.pending);
			}
			result.pending = Frame{cursor, 0, false};
			result.has_vcl = false;
			started = true;
		} else if (!started) {
			result.stop = Stop::invalid; break;
		}
		const bool vcl = hevc ? type <= 31 : type <= 5;
		result.has_vcl = result.has_vcl || vcl;
		result.pending.key = result.pending.key || (hevc ? (type >= 16 && type <= 21) : type == 5);
		result.end += 4 + length;
	}
	return result;
}

// A plausible PCM region must agree with both sample alignment and video time.
// The small tolerance accommodates Sony's rounded nominal frame rates; actual
// bytes, rather than a repeated rounded duration, determine audio sample counts.
inline bool audioMatches(std::size_t bytes, std::size_t frames, int bytes_per_sample,
	int sample_rate, int video_duration, int video_timescale) {
	if (!frames || bytes_per_sample <= 0 || sample_rate <= 0 || video_duration <= 0 ||
		video_timescale <= 0 || bytes % bytes_per_sample) return false;
	const long double expected = static_cast<long double>(frames) * video_duration *
		sample_rate / video_timescale;
	const long double actual = bytes / bytes_per_sample;
	return std::fabs(actual - expected) <= std::max(1.0L, expected * 0.01L);
}

} // namespace rsv
