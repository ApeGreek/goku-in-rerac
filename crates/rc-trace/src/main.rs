//! `rc-trace`: compare our reimplementation against the PS2's EE memory. Doc: docs/plan/trace_harness.md.

use anyhow::{bail, Context, Result};
use rc_trace::ee::{self, EeSource, Savestate};
use rc_trace::tfrag_light_cmp::{self as tlc, LevelInputs};
use rc_trace::novalis_spawn as ns;
use rc_trace::{default_extracted, pine};
use std::path::PathBuf;

const USAGE: &str = "\
rc-trace - PCSX2 ground-truth harness (docs/plan/trace_harness.md)

EE memory source (for the commands that read RAM), one of:
  --state <file.p2s | dir | latest>   PCSX2 savestate; 'latest' = newest SCUS-97199*.p2s in
                                      ~/Library/Application Support/PCSX2/sstates
  --ee <eeMemory.bin>                 raw 32 MiB EE RAM dump
  --pine [slot]                       live read over PCSX2's PINE socket (default slot 28011)

Commands:
  compare-tfrag-light <source> [--level 01] [--extracted DIR] [--csv FILE] [--max-diffs N]
      Lights every tfrag with rc_formats::tfrag_light and compares with the RGBA blocks in RAM.
      Exit status 0 = all equal, 2 = mismatches, 1 = error.
  compare-novalis-spawn <source> [--extracted DIR] [--no-jump] [--ticks N] [--max-diffs N] [--out FILE]
      A savestate taken on Novalis after the landing, standing at the spawn: game-state chunks, hero block,
      follow camera, fog globals, rand state + tick counter, static moby table, tie/shrub lit colours,
      each against the port (rc_game run headless for the tick counter's ticks). Report text to
      extracted/traces/novalis_spawn_report.txt, moby mismatches to novalis_spawn_mobys.csv.
      --no-jump: no scripted ✕ (default: one ✕ tap at the tick the idle counter 0x160ff0 points at).
      --press-ticks N: the scripted ✕ is held N ticks, ending at the idle counter's tick (default 2: the savestate's landing timers match a 2-3 tick press).
      --cutscene-ticks N: the first N port ticks run in game mode 2 (default: ticks − frames in mode 0).
      --load-pre-draws N: extra rand() draws right before the load pass (diagnostic; HeroInit's own draw is in).
      --load-emitters-visible: the load pass's class-27 emitters see the camera (diagnostic: the pre-fix port,
      one particle each; the port's rule is the game's unbuilt view, which culls them).
      --no-audio: run the port without the sound layer (diagnostic: no sound draws on the game stream).
  port-load-pass --out FILE [--load-pre-draws N] [--load-emitters-visible]
      Diagnostic: the port's static mobys right after its load pass, as CSV.
  compare-tie-shrub-light <source> [--extracted DIR] [--max-diffs N]
      Only the tie/shrub lit-colour part (level 01).
  info --state <p2s>                  list savestate entries and version
  dump-ee <source> --out FILE [--entry NAME]   write EE RAM (or any savestate entry) to FILE
  find <source> --bytes HEX [--align N] [--max N]   list EE addresses holding the bytes
  read <source> --addr HEX --len N    hex-dump EE memory
  pine-info [slot]                    print PCSX2 version / game / status over PINE
  pine-savestate <state-slot> [--slot N]   ask PCSX2 to save to a state slot (like pressing F1)
  synth [--level 01] [--unlit] --out FILE [--base HEX]   write a synthetic 'loaded level' EE image
";

struct Args(Vec<String>);

impl Args {
    fn flag(&mut self, name: &str) -> bool {
        match self.0.iter().position(|a| a == name) {
            Some(i) => { self.0.remove(i); true }
            None => false,
        }
    }
    fn opt(&mut self, name: &str) -> Result<Option<String>> {
        let Some(i) = self.0.iter().position(|a| a == name) else { return Ok(None) };
        if i + 1 >= self.0.len() { bail!("{name} needs a value"); }
        let v = self.0.remove(i + 1);
        self.0.remove(i);
        Ok(Some(v))
    }
    /// `--pine` with an optional numeric slot after it.
    fn pine(&mut self) -> Option<u16> {
        let i = self.0.iter().position(|a| a == "--pine")?;
        self.0.remove(i);
        if let Some(s) = self.0.get(i).and_then(|s| s.parse().ok()) { self.0.remove(i); Some(s) } else { Some(pine::DEFAULT_SLOT) }
    }
    fn source(&mut self) -> Result<EeSource> {
        if let Some(s) = self.opt("--state")? { return Ok(EeSource::State(ee::resolve_state(&s)?)); }
        if let Some(p) = self.opt("--ee")? { return Ok(EeSource::Raw(p.into())); }
        if let Some(slot) = self.pine() { return Ok(EeSource::Pine(slot)); }
        bail!("need an EE memory source: --state, --ee or --pine\n\n{USAGE}")
    }
    fn level(&mut self) -> Result<u32> { Ok(self.opt("--level")?.map(|l| l.parse()).transpose().context("--level")?.unwrap_or(1)) }
    fn extracted(&mut self) -> Result<PathBuf> { Ok(self.opt("--extracted")?.map(PathBuf::from).unwrap_or_else(default_extracted)) }
    fn done(&self) -> Result<()> {
        if !self.0.is_empty() { bail!("unexpected arguments: {:?}\n\n{USAGE}", self.0); }
        Ok(())
    }
}

fn hex_u32(s: &str) -> Result<u32> { u32::from_str_radix(s.trim_start_matches("0x"), 16).with_context(|| format!("bad hex {s:?}")) }

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("error: {e:#}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<i32> {
    let mut a = Args(std::env::args().skip(1).collect());
    if a.0.is_empty() || a.flag("--help") || a.flag("-h") {
        print!("{USAGE}");
        return Ok(0);
    }
    let cmd = a.0.remove(0);
    match cmd.as_str() {
        "compare-tfrag-light" => compare_tfrag_light(a),
        "compare-novalis-spawn" => compare_novalis_spawn(a),
        "port-load-pass" => {
            // Diagnostic: the port's static mobys right after its load pass (no savestate needed).
            let extracted = a.extracted()?;
            let pre: u32 = a.opt("--load-pre-draws")?.map(|s| s.parse()).transpose()?.unwrap_or(0);
            let visible = a.flag("--load-emitters-visible");
            let out = PathBuf::from(a.opt("--out")?.context("--out required")?);
            a.done()?;
            let lv = rc_trace::port_sim::LevelData::load(&extracted, 1)?;
            let sim = rc_trace::port_sim::PortSim::with_options(&lv, &rc_trace::port_sim::SimOptions { load_pre_draws: pre, load_emitters_visible: visible, ..Default::default() })?;
            let mut s = String::from("index,instance,o_class,state,mode,x,y,z,rx,ry,rz,pvar55,seq_b,frame_b\n");
            for (i, m) in sim.game.mobys.mobys.iter().enumerate().take(sim.n_static) {
                s += &format!("{i},{},{},{},{},{},{},{},{},{},{},{},{},{}\n", sim.moby_to_instance[i], m.o_class, m.state, m.mode, m.position[0], m.position[1], m.position[2],
                    m.rotation[0], m.rotation[1], m.rotation[2], m.pvars.get(0x55).copied().unwrap_or(0), m.anim.seq_b, m.anim.frame_b);
            }
            std::fs::write(&out, s)?;
            println!("wrote {} (load pass: {} draws, rng {:#010x})", out.display(), sim.load_draws, sim.game.rng.state);
            Ok(0)
        }
        "compare-tie-shrub-light" => {
            let extracted = a.extracted()?;
            let max: usize = a.opt("--max-diffs")?.map(|s| s.parse()).transpose()?.unwrap_or(20);
            let src = a.source()?;
            a.done()?;
            let img = src.load()?;
            let mut out = ns::Out::default();
            ns::check_tie_shrub(&img, &extracted, &mut out, max)?;
            Ok(if out.tallies.iter().all(|t| t.1 == t.2) { 0 } else { 2 })
        }
        "info" => {
            let p = ee::resolve_state(&a.opt("--state")?.context("--state required")?)?;
            a.done()?;
            let s = Savestate::open(&p)?;
            println!("{}", p.display());
            if let Ok(v) = s.version() { println!("  save version {:#010x} (major {:#06x}), PCSX2 {}", v.save_version, v.save_version >> 16, v.build); }
            for e in &s.archive.entries {
                println!("  {:<34} method {:>2}  {:>10} -> {:>10} bytes", e.name, e.method, e.compressed_size, e.size);
            }
            Ok(0)
        }
        "dump-ee" => {
            let out = PathBuf::from(a.opt("--out")?.context("--out required")?);
            let entry = a.opt("--entry")?;
            let src = a.source()?;
            a.done()?;
            let bytes = match (entry, &src) {
                (Some(e), EeSource::State(p)) => Savestate::open(p)?.archive.read(&e)?,
                (Some(_), _) => bail!("--entry needs --state"),
                (None, s) => s.load()?.ram,
            };
            if let Some(d) = out.parent() { std::fs::create_dir_all(d)?; }
            std::fs::write(&out, &bytes)?;
            println!("wrote {} ({} bytes)", out.display(), bytes.len());
            Ok(0)
        }
        "find" => {
            let hex = a.opt("--bytes")?.context("--bytes required")?;
            let align: usize = a.opt("--align")?.map(|s| s.parse()).transpose()?.unwrap_or(2);
            let max: usize = a.opt("--max")?.map(|s| s.parse()).transpose()?.unwrap_or(64);
            let src = a.source()?;
            a.done()?;
            let clean: String = hex.chars().filter(|c| c.is_ascii_hexdigit()).collect();
            if !clean.len().is_multiple_of(2) || clean.is_empty() { bail!("--bytes needs an even number of hex digits"); }
            let pat: Vec<u8> = (0..clean.len()).step_by(2).map(|i| u8::from_str_radix(&clean[i..i + 2], 16).unwrap()).collect();
            let img = src.load()?;
            let hits = img.find(&pat, align);
            println!("{} hit(s) for {} bytes (align {align})", hits.len(), pat.len());
            for h in hits.iter().take(max) { println!("  {h:#010x}"); }
            Ok(0)
        }
        "read" => {
            let addr = hex_u32(&a.opt("--addr")?.context("--addr required")?)?;
            let len: usize = a.opt("--len")?.map(|s| s.parse()).transpose()?.unwrap_or(0x40);
            let src = a.source()?;
            a.done()?;
            let img = src.load()?;
            for (k, row) in img.bytes(addr, len)?.chunks(16).enumerate() {
                let hex: Vec<String> = row.iter().map(|b| format!("{b:02x}")).collect();
                println!("{:#010x}: {}", addr as usize + 16 * k, hex.join(" "));
            }
            Ok(0)
        }
        "pine-info" => {
            let slot = a.0.first().and_then(|s| s.parse().ok()).unwrap_or(pine::DEFAULT_SLOT);
            let mut c = pine::Pine::connect(slot)?;
            println!("socket  {}", pine::socket_path(slot).display());
            println!("version {}", c.version()?);
            println!("status  {}", c.status_name());
            println!("game    {} | {}", c.game_id().unwrap_or_default(), c.title().unwrap_or_default());
            Ok(0)
        }
        "pine-savestate" => {
            let slot: u16 = a.opt("--slot")?.map(|s| s.parse()).transpose()?.unwrap_or(pine::DEFAULT_SLOT);
            let state: u8 = a.0.first().context("state slot number required")?.parse()?;
            pine::Pine::connect(slot)?.save_state(state)?;
            println!("requested save to state slot {state}; the file appears in {}", ee::pcsx2_sstates_dir().unwrap_or_default().display());
            Ok(0)
        }
        "synth" => {
            let level = a.level()?;
            let extracted = a.extracted()?;
            let unlit = a.flag("--unlit");
            let base = a.opt("--base")?.map(|s| hex_u32(&s)).transpose()?.unwrap_or(0x0040_0000);
            let out = PathBuf::from(a.opt("--out")?.context("--out required")?);
            a.done()?;
            let lvl = LevelInputs::load(&extracted, level)?;
            let img = tlc::synthesize(&lvl, base, !unlit);
            if let Some(d) = out.parent() { std::fs::create_dir_all(d)?; }
            std::fs::write(&out, &img.ram)?;
            println!("wrote {} ({})", out.display(), img.source);
            Ok(0)
        }
        _ => bail!("unknown command {cmd:?}\n\n{USAGE}"),
    }
}

fn compare_tfrag_light(mut a: Args) -> Result<i32> {
    let level = a.level()?;
    let extracted = a.extracted()?;
    let csv = a.opt("--csv")?.map(PathBuf::from).unwrap_or_else(|| extracted.join(format!("traces/level{level:02}_tfrag_light_mismatches.csv")));
    let max_diffs: usize = a.opt("--max-diffs")?.map(|s| s.parse()).transpose()?.unwrap_or(20);
    let src = a.source()?;
    a.done()?;

    let lvl = LevelInputs::load(&extracted, level)?;
    let img = src.load()?;
    println!("level {level:02}: {} tfrags, {} directional light sets (disc); EE image: {}", lvl.tfrags.len(), lvl.bank.count, img.source);

    let loc = tlc::locate(&img, &lvl)?;
    println!(
        "tfrag header table at EE {:#010x} (block {:#010x}); {} candidate(s) for tfrag 0's bsphere, {}/{} headers equal on load-invariant bytes",
        loc.table, loc.table - lvl.table_offset as u32, loc.candidates, loc.headers_matching, lvl.tfrags.len()
    );
    println!("  data pointers relocated to absolute: {}/{} (still disc-relative: {})", loc.relocated, lvl.tfrags.len(), loc.unrelocated);
    if let Some((g, v, implied, ok)) = loc.core_ptr_check {
        println!("  level global {g:#x} (core-data base) = {v:#010x} -> tfrags block {implied:#010x}: {}", if ok { "agrees with the search" } else { "DISAGREES with the search" });
    }
    let vol: Vec<String> = loc.volatile_diffs.iter().filter(|d| d.2 > 0).map(|(o, n, c)| format!("{n}(0x{o:02x}) x{c}")).collect();
    if !vol.is_empty() { println!("  header bytes changed at run time vs disc: {}", vol.join(", ")); }
    if loc.relocated * 2 < lvl.tfrags.len() {
        println!("  WARNING: most data pointers are not relocated; the level may not have finished loading");
    }

    let bank = tlc::locate_bank(&img, &lvl);
    match &bank {
        Some(b) => println!("directional light bank at EE {:#010x} ({}): {}", b.addr, b.how, if b.equals_disc { "equals the disc gameplay bank" } else { "DIFFERS from the disc gameplay bank" }),
        None => println!("directional light bank: not found (tfrags with point lights will be skipped)"),
    }
    let points = bank.as_ref().map(|b| &b.points);

    println!("\nours (disc light bank) vs RAM:");
    let r = tlc::compare(&img, &lvl, &loc, &lvl.bank, points)?;
    r.print(max_diffs);
    if !r.diffs.is_empty() {
        r.write_csv(&csv)?;
        println!("  all {} mismatching vertices written to {}", r.diffs.len(), csv.display());
    }
    let mut ok = r.all_equal();
    if let Some(b) = bank.as_ref().filter(|b| !b.equals_disc) {
        println!("\nours (light bank read from RAM) vs RAM:");
        let r2 = tlc::compare(&img, &lvl, &loc, &b.bank, points)?;
        r2.print(max_diffs);
        ok = r2.all_equal();
    }
    println!("\nresult: {}", if ok { "MATCH - every lit vertex byte equals the game's" } else { "MISMATCH" });
    Ok(if ok { 0 } else { 2 })
}

fn compare_novalis_spawn(mut a: Args) -> Result<i32> {
    let extracted = a.extracted()?;
    let no_jump = a.flag("--no-jump");
    let ticks: Option<u64> = a.opt("--ticks")?.map(|s| s.parse()).transpose().context("--ticks")?;
    let max: usize = a.opt("--max-diffs")?.map(|s| s.parse()).transpose()?.unwrap_or(20);
    let pre_draws: u32 = a.opt("--load-pre-draws")?.map(|s| s.parse()).transpose().context("--load-pre-draws")?.unwrap_or(0);
    let emitters_visible = a.flag("--load-emitters-visible");
    let no_audio = a.flag("--no-audio");
    let hold: u64 = a.opt("--press-ticks")?.map(|s| s.parse()).transpose().context("--press-ticks")?.unwrap_or(2);
    let cutscene: Option<u64> = a.opt("--cutscene-ticks")?.map(|s| s.parse()).transpose().context("--cutscene-ticks")?;
    let report = a.opt("--out")?.map(PathBuf::from).unwrap_or_else(|| extracted.join("traces/novalis_spawn_report.txt"));
    let src = a.source()?;
    a.done()?;
    let img = src.load()?;
    let mut out = ns::Out::default();
    let level = img.u32(ns::LEVEL)?;
    if level != 1 { bail!("level global 0x15ed84 = {level}: this savestate is not on Novalis"); }
    let mut tl = ns::Timeline::read(&img)?;
    if let Some(t) = ticks { tl.ticks = t; }
    let vsync = img.u32(ns::VSYNC)?;
    let play = img.u32(ns::PLAY_TIME)?;
    println!("EE image: {}", img.source);
    println!(
        "timeline: tick counter 0x15f5cc = {} (port ticks {}), mode 0x15f5c4 = {} for {} frames, idle counter 0x160ff0 = {} (last pad input at port tick {:?}), vsync 0x15f3f8 = {vsync}, play time 0x15eea4 = {play}",
        tl.tick_counter, tl.ticks, tl.mode, tl.mode_frames, tl.idle, tl.press_at
    );
    let gs = ns::port_game_state(&extracted)?;
    ns::check_game_state(&img, &gs, &mut out)?;
    let lv = rc_trace::port_sim::LevelData::load(&extracted, 1)?;
    let t0 = std::time::Instant::now();
    let opt = rc_trace::port_sim::SimOptions { load_pre_draws: pre_draws, load_emitters_visible: emitters_visible, cutscene_ticks: cutscene.unwrap_or(tl.ticks.saturating_sub(tl.mode_frames.max(0) as u64)), no_audio };
    println!("port options: {opt:?}");
    let hold = if no_jump { 0 } else { hold };
    let sim = ns::run_port_hold(&lv, &tl, hold, &opt)?;
    println!("\n(port: load pass + {} ticks in {:.1} s{})", tl.ticks, t0.elapsed().as_secs_f64(), if no_jump { String::new() } else { format!(", ✕ held {hold} tick(s) ending at port tick {:?}", tl.press_at) });
    ns::check_hero(&img, &sim, &mut out)?;
    ns::check_camera(&img, &sim, &mut out)?;
    ns::check_fog(&img, &lv, &sim, &mut out)?;
    ns::check_rng(&img, &sim, &tl, &mut out)?;
    ns::check_mobys(&img, &sim, &mut out, &extracted.join("traces/novalis_spawn_mobys.csv"))?;
    ns::check_tie_shrub(&img, &extracted, &mut out, max)?;
    println!("\n== summary (matching / total)");
    for (name, ok, total) in &out.tallies {
        println!("  {:<46} {:>7} / {:<7} {}", name, ok, total, if ok == total { "ok" } else { "MISMATCH" });
    }
    if let Some(d) = report.parent() { std::fs::create_dir_all(d)?; }
    std::fs::write(&report, &out.text)?;
    println!("report written to {}", report.display());
    Ok(if out.tallies.iter().all(|t| t.1 == t.2) { 0 } else { 2 })
}
