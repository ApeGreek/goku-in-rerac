//! Integration tests: the moby class ports: common and cheap classes, creatures, breakables, bolts and path classes (docs/workflows/testing.md §3).
//! One module per former test file; run one with `cargo xtask test-job --test classes --filter <module>::`.

#[path = "../common/mod.rs"]
mod common;

mod bolt_crank_novalis;
mod breakables_levels;
mod cheap_classes_a;
mod cheap_classes_b;
mod common_classes_levels;
mod creature_classes;
mod creatures_enemies_novalis;
mod creatures_novalis;
mod enemy_units;
mod explosion_survey;
mod gold_bolt_infobot_novalis;
mod help_directors;
mod moby_update_novalis;
mod path_classes;
mod quick_wins;
