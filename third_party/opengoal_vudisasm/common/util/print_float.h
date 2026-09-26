#pragma once
#include <string>
#include <cstdio>
inline std::string float_to_string(float v, bool = true) { char b[64]; std::snprintf(b, sizeof b, "%.9g", double(v)); return b; }
