find_package(ZLIB REQUIRED)
add_library(rc_core STATIC
  buffer.cpp
  iso9660.cpp
  wad.cpp
  toc.cpp
  level.cpp
  level_core.cpp
  texture.cpp
  vif.cpp
  tfrag.cpp
  tie.cpp
  shrub.cpp
  sky.cpp
  moby.cpp
  gadget.cpp
  collision.cpp
  occlusion.cpp
  particle.cpp
  hud.cpp
  sound.cpp
)
target_include_directories(rc_core PUBLIC ${CMAKE_SOURCE_DIR}/src)
target_link_libraries(rc_core PUBLIC ZLIB::ZLIB)
