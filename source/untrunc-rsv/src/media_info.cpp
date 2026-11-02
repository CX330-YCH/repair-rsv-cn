#include "media_info.h"
#include "rsv_parser.h"
#include <sys/stat.h>
#include <cctype>
#include <cstdio>
#include <iomanip>
#include <iostream>
#include <sstream>

extern "C" {
#include <libavformat/avformat.h>
#include <libavutil/pixdesc.h>
}

namespace {
std::string quote(const std::string& value) {
	std::ostringstream out;
	out << '"';
	for (unsigned char c : value) {
		if (c == '"' || c == '\\') out << '\\' << c;
		else if (c < 32) out << "\\u" << std::hex << std::setw(4) << std::setfill('0') << int(c);
		else out << c;
	}
	out << '"';
	return out.str();
}

void duration(std::ostream& out, const AVFormatContext* format, const AVStream* stream) {
	if (stream->duration != AV_NOPTS_VALUE && stream->duration >= 0)
		out << stream->duration * av_q2d(stream->time_base);
	else if (format->duration != AV_NOPTS_VALUE && format->duration >= 0)
		out << double(format->duration) / AV_TIME_BASE;
	else out << "null";
}
}

int printMediaInfo(const std::string& path) {
	struct stat file_stat;
	if (stat(path.c_str(), &file_stat) || !S_ISREG(file_stat.st_mode)) {
		std::cerr << "Cannot read media file\n";
		return 1;
	}
	FILE* file = std::fopen(path.c_str(), "rb");
	if (!file) { std::cerr << "Cannot open media file\n"; return 1; }
	unsigned char header[12] = {};
	const size_t read = std::fread(header, 1, sizeof(header), file);
	std::fclose(file);
	const bool raw_rsv = rsv::metadata(header, read);
	std::cout << std::setprecision(12) << "{\"sizeBytes\":" << file_stat.st_size;
	if (raw_rsv) {
		// RSV can omit SPS/PPS and all container timing. Never attribute the
		// donor's dimensions or frame rate to this raw file as measured facts.
		std::cout << ",\"container\":\"RSV\",\"unindexed\":true,\"readable\":false,\"video\":null,\"audio\":[]}\n";
		return 0;
	}
	av_log_set_level(AV_LOG_ERROR);
	AVFormatContext* input = nullptr;
	AVDictionary* options = nullptr;
	av_dict_set(&options, "probesize", "8388608", 0);
	av_dict_set(&options, "analyzeduration", "2000000", 0);
	int result = avformat_open_input(&input, path.c_str(), nullptr, &options);
	av_dict_free(&options);
	if (result < 0 || avformat_find_stream_info(input, nullptr) < 0) {
		avformat_close_input(&input);
		std::cout << ",\"container\":\"Unknown\",\"unindexed\":false,\"readable\":false,\"video\":null,\"audio\":[]}\n";
		return 0;
	}
	std::string container = input->iformat->name;
	if (container.find("mov") != std::string::npos) {
		const auto dot = path.find_last_of('.');
		container = dot == std::string::npos ? "MP4 / MOV" : path.substr(dot + 1);
		for (char& c : container) c = std::toupper(static_cast<unsigned char>(c));
	}
	std::cout << ",\"container\":" << quote(container) << ",\"unindexed\":false,\"readable\":true,\"video\":";
	bool has_video = false;
	for (unsigned i = 0; i < input->nb_streams; ++i) {
		const AVStream* stream = input->streams[i];
		const AVCodecParameters* codec = stream->codecpar;
		if (codec->codec_type != AVMEDIA_TYPE_VIDEO) continue;
		const AVRational fps = stream->avg_frame_rate.num > 0 ? stream->avg_frame_rate : stream->r_frame_rate;
		const auto* pixel = av_pix_fmt_desc_get(static_cast<AVPixelFormat>(codec->format));
		const int depth = codec->bits_per_raw_sample > 0 ? codec->bits_per_raw_sample :
			(pixel ? pixel->comp[0].depth : 0);
		std::cout << "{\"codec\":" << quote(avcodec_get_name(codec->codec_id))
			<< ",\"width\":" << codec->width << ",\"height\":" << codec->height << ",\"fps\":";
		if (fps.num > 0 && fps.den > 0) std::cout << av_q2d(fps); else std::cout << "null";
		std::cout << ",\"durationSeconds\":";
		duration(std::cout, input, stream);
		std::cout << ",\"frameCount\":";
		if (stream->nb_frames > 0) std::cout << stream->nb_frames; else std::cout << "null";
		std::cout << ",\"pixelFormat\":" << (pixel ? quote(pixel->name) : "null")
			<< ",\"bitDepth\":" << depth << '}';
		has_video = true;
		break;
	}
	if (!has_video) std::cout << "null";
	std::cout << ",\"audio\":[";
	bool first = true;
	for (unsigned i = 0; i < input->nb_streams; ++i) {
		const AVStream* stream = input->streams[i];
		const AVCodecParameters* codec = stream->codecpar;
		if (codec->codec_type != AVMEDIA_TYPE_AUDIO) continue;
		if (!first) std::cout << ',';
		first = false;
		const int depth = codec->bits_per_raw_sample > 0 ? codec->bits_per_raw_sample : codec->bits_per_coded_sample;
		std::cout << "{\"codec\":" << quote(avcodec_get_name(codec->codec_id))
			<< ",\"sampleRate\":" << codec->sample_rate << ",\"channels\":";
#if LIBAVCODEC_VERSION_MAJOR >= 59
		std::cout << codec->ch_layout.nb_channels;
#else
		std::cout << codec->channels;
#endif
		std::cout << ",\"bitDepth\":" << depth << ",\"durationSeconds\":";
		duration(std::cout, input, stream);
		std::cout << '}';
	}
	std::cout << "]}\n";
	avformat_close_input(&input);
	return 0;
}
