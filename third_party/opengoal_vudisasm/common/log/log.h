#pragma once
#include <cstdio>
#include "fmt/format.h"
namespace lg {
template <typename... A> void error(fmt::format_string<A...> f, A&&... a) { std::fprintf(stderr, "error: %s\n", fmt::format(f, std::forward<A>(a)...).c_str()); }
template <typename... A> void warn(fmt::format_string<A...> f, A&&... a) { std::fprintf(stderr, "warn: %s\n", fmt::format(f, std::forward<A>(a)...).c_str()); }
template <typename... A> void info(fmt::format_string<A...> f, A&&... a) { std::fprintf(stderr, "info: %s\n", fmt::format(f, std::forward<A>(a)...).c_str()); }
template <typename... A> void print(fmt::format_string<A...> f, A&&... a) { std::fprintf(stderr, "%s\n", fmt::format(f, std::forward<A>(a)...).c_str()); }
template <typename... A> void die(fmt::format_string<A...> f, A&&... a) { std::fprintf(stderr, "fatal: %s\n", fmt::format(f, std::forward<A>(a)...).c_str()); std::abort(); }
}
