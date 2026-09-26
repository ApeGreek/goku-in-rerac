# Unit tests that do not need game data. Tests that need the ISO are skipped
# unless RC_ISO is set in the environment.
add_executable(rc_tests
  test_main.cpp
  test_buffer.cpp
  test_wad.cpp
)
target_link_libraries(rc_tests PRIVATE rc_core)
add_test(NAME rc_tests COMMAND rc_tests)
