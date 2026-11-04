#![cfg_attr(windows, windows_subsystem = "windows")]

mod gui_common;

use eframe::egui::{self, Align2, Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use gui_common::*;
use kiseki_tool::ed7save::{self as save, Profile, field};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const TITLE: &str = concat!("Crossbell Save Editor ", env!("VER_CROSSBELLSAVEEDITOR"), env!("VER_SUFFIX"));

fn title() -> &'static str {
	TITLE
}

const GAMES: [(&str, &str); 2] = [("zero", "Trails from Zero"), ("azure", "Trails to Azure")];

fn profile(game: usize) -> &'static Profile {
	if game == 1 { &save::AZURE } else { &save::ZERO }
}

fn num_hint() -> &'static str {
	"Drag, or click and type a number"
}

const SEPITH: [&str; 7] = ["Earth", "Water", "Fire", "Wind", "Time", "Space", "Mirage"];

fn atlas_uv(t: &egui::TextureHandle, c: u32) -> Rect {
	let s = t.size();
	let (w, h) = (64.0 / s[0] as f32, 64.0 / s[1] as f32);
	Rect::from_min_size(Pos2::new((c % 16) as f32 * w, (c / 16) as f32 * h), Vec2::new(w, h))
}

fn help_icon(ui: &mut egui::Ui, p: &Pal, tip: impl FnOnce(&mut egui::Ui)) {
	let (r, resp) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
	let col = if resp.hovered() { p.accent } else { p.muted };
	ui.painter().circle_stroke(r.center(), 7.5, Stroke::new(1.2, col));
	ui.painter().text(r.center() + Vec2::new(0.0, 0.5), Align2::CENTER_CENTER, "?", f_semi(11.5), col);
	if resp.hovered() {
		egui::Area::new(resp.id.with("help"))
			.order(egui::Order::Tooltip)
			.fixed_pos(r.right_top() + Vec2::new(8.0, 0.0))
			.constrain(true)
			.interactable(false)
			.show(ui.ctx(), |ui| egui::Frame::popup(ui.style()).show(ui, tip));
	}
}

fn tip_table(ui: &mut egui::Ui, p: &Pal, title: &str, head: [&str; 2], rows: &[[String; 2]], cur: Option<usize>) {
	ui.label(egui::RichText::new(title).font(f_semi(13.5)).color(p.text));
	ui.add_space(4.0);
	egui::Grid::new(("tip", title)).num_columns(2).spacing([28.0, 3.0]).show(ui, |ui| {
		for h in head {
			ui.label(egui::RichText::new(h).font(f_body(12.0)).color(p.muted));
		}
		ui.end_row();
		for (i, row) in rows.iter().enumerate() {
			let col = if cur == Some(i) { p.accent } else { p.text };
			for (j, c) in row.iter().enumerate() {
				let t = egui::RichText::new(c).font(if j == 0 { f_semi(13.0) } else { f_body(13.0) }).color(col);
				if j == 0 {
					ui.label(t);
				} else {
					ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| ui.label(t));
				}
			}
			ui.end_row();
		}
	});
}

fn sepith_icon(ui: &mut egui::Ui, atlas: &Option<egui::TextureHandle>, k: usize, size: f32) {
	match atlas {
		Some(t) => {
			let r = atlas_uv(t, save::CELL_SEPITH + k as u32);
			let (w, h) = (r.width() / 64.0, r.height() / 64.0);
			let uv = Rect::from_min_size(r.min + Vec2::new(0.0, 12.0 * h), Vec2::new(40.0 * w, 40.0 * h));
			ui.add(egui::Image::new(t).uv(uv).fit_to_exact_size(Vec2::splat(size)));
		}
		None => {
			ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
		}
	}
}

fn atlas_cell(ui: &mut egui::Ui, atlas: &Option<egui::TextureHandle>, cell: Option<u32>, size: f32) {
	match (atlas, cell) {
		(Some(t), Some(c)) => {
			ui.add(egui::Image::new(t).uv(atlas_uv(t, c)).fit_to_exact_size(Vec2::splat(size)));
		}
		_ => {
			ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
		}
	}
}

struct Slot {
	name: String,
	dir: PathBuf,
	title: String,
	sub: String,
	when: String,
	playtime: String,
	difficulty: String,
}

fn info_str(b: &[u8], o: usize) -> String {
	let Some(s) = b.get(o..) else { return String::new() };
	let e = s.iter().position(|&c| c == 0 || c == b'\n').unwrap_or(s.len());
	cp932::decode_lossy(&s[..e]).replace(['\u{301C}', '\u{FF5E}'], "~").trim().to_string()
}

fn read_slots(root: &Path) -> Vec<Slot> {
	let mut v: Vec<Slot> = std::fs::read_dir(root)
		.into_iter()
		.flatten()
		.flatten()
		.filter(|e| e.path().join("savedata.dat").is_file())
		.filter_map(|e| {
			let name = e.file_name().to_string_lossy().to_string();
			if !name.starts_with("data") {
				return None;
			}
			let info = std::fs::read(e.path().join("info.txt")).unwrap_or_default();
			let head = info_str(&info, 0);
			let (title, difficulty) = head.rsplit_once('\u{3000}').map(|(a, b)| (a.trim().to_string(), b.trim().to_string())).unwrap_or((head.clone(), String::new()));
			if title.is_empty() && difficulty.is_empty() || head.contains("System") || head.contains("\u{421}\u{438}\u{441}\u{442}\u{435}\u{43c}") {
				return None;
			}
			let line = info_str(&info, 0x80);
			let playtime = line.rsplit(' ').next().filter(|t| t.matches(':').count() == 2).unwrap_or("").to_string();
			let sub = line.split("  ").next().unwrap_or("").trim().to_string();
			Some(Slot { name, dir: e.path(), title, sub, when: info_str(&info, 0x480), playtime, difficulty })
		})
		.collect();
	v.sort_by(|a, b| a.name.cmp(&b.name));
	v
}

fn default_dir(game: usize) -> String {
	std::env::var_os("USERPROFILE").map(|h| PathBuf::from(h).join("Saved Games").join("Falcom").join(profile(game).falcom)).filter(|p| p.is_dir()).map(|p| p.display().to_string()).unwrap_or_default()
}

fn stamp() -> String {
	let s = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
	let (days, rem) = (s.div_euclid(86400), s.rem_euclid(86400));
	let z = days + 719_468;
	let era = z.div_euclid(146_097);
	let doe = z - era * 146_097;
	let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
	let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
	let mp = (5 * doy + 2) / 153;
	let d = doy - (153 * mp + 2) / 5 + 1;
	let m = if mp < 10 { mp + 3 } else { mp - 9 };
	let y = yoe + era * 400 + i64::from(m <= 2);
	format!("{y:04}-{m:02}-{d:02}_{:02}-{:02}-{:02}", rem / 3600, rem % 3600 / 60, rem % 60)
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
	std::fs::create_dir_all(to)?;
	for e in std::fs::read_dir(from)?.flatten() {
		if e.path().is_file() {
			std::fs::copy(e.path(), to.join(e.file_name()))?;
		}
	}
	Ok(())
}

fn card_size(big: bool) -> Vec2 {
	if big { Vec2::new(160.0, 150.0) } else { Vec2::new(140.0, 124.0) }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum LPos {
	Line(usize),
	Avail(usize),
}

fn lineup_swap(list: &mut [u32], have: &[u32], a: LPos, b: LPos) {
	match (a, b) {
		(LPos::Line(i), LPos::Line(j)) => list.swap(i, j),
		(LPos::Line(i), LPos::Avail(j)) | (LPos::Avail(j), LPos::Line(i)) => list[i] = have[j],
		(LPos::Avail(_), LPos::Avail(_)) => {}
	}
}

fn lineup_move(list: &mut Vec<u32>, have: &[u32], a: LPos, zone: u8) {
	match (a, zone) {
		(LPos::Line(i), 2) if list.len() > 1 => {
			list.remove(i);
		}
		(LPos::Line(i), 1) => {
			let c = list.remove(i);
			list.push(c);
		}
		(LPos::Avail(_), 0) if list.len() >= save::PARTY_ACTIVE => {}
		(LPos::Avail(j), 0 | 1) if list.len() < save::PARTY => list.push(have[j]),
		_ => {}
	}
}

#[derive(Clone)]
enum Act {
	Overwrite,
	Close,
	Open(usize),
}

struct App {
	pal: Pal,
	st: Settings,
	dir: String,
	slots: Vec<Slot>,
	sel: Option<usize>,
	data: Option<Vec<u8>>,
	orig: Vec<u8>,
	tab: usize,
	item_tab: usize,
	lsel: Option<LPos>,
	drag_src: Option<LPos>,
	chr_sel: usize,
	pick_game: Option<usize>,
	search: String,
	owned_only: bool,
	collapsed: std::collections::HashSet<String>,
	game: Option<usize>,
	g: &'static Profile,
	items: Vec<save::Item>,
	item_of: HashMap<u32, usize>,
	names: HashMap<u32, String>,
	crafts: HashMap<u32, save::Craft>,
	orbs: HashMap<usize, save::OrbLayout>,
	mods: HashMap<u32, Vec<save::Mod>>,
	slot_costs: save::SlotCosts,
	faces: HashMap<u32, Vec<u8>>,
	face_tex: HashMap<u32, Option<egui::TextureHandle>>,
	art_tex: HashMap<String, Option<egui::TextureHandle>>,
	icons: Option<Option<egui::TextureHandle>>,
	status: String,
	dialog: Dialog<Act>,
	zoom: f32,
	about: bool,
	tips: bool,
	allow_close: bool,
	scan_at: f64,
	scan_sig: u64,
	scan_dir: String,
	gsearch: String,
	gsearch_open: bool,
	scroll_sel: bool,
	shown_title: String,
}

fn slot_label(name: &str) -> String {
	match name.strip_prefix("data").and_then(|n| n.parse::<u32>().ok()) {
		Some(n @ 1..=199) => format!("{name} · slot {n}"),
		_ => name.to_string(),
	}
}

fn dir_signature(root: &Path) -> u64 {
	use std::hash::{Hash, Hasher};
	let mut h = std::collections::hash_map::DefaultHasher::new();
	let mut v: Vec<(String, u64)> = std::fs::read_dir(root)
		.into_iter()
		.flatten()
		.flatten()
		.filter_map(|e| {
			let m = std::fs::metadata(e.path().join("savedata.dat")).ok()?;
			let t = m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
			Some((e.file_name().to_string_lossy().to_string(), t))
		})
		.collect();
	v.sort();
	v.hash(&mut h);
	h.finish()
}

impl App {
	fn new(cc: &eframe::CreationContext) -> App {
		setup_fonts(&cc.egui_ctx);
		egui_extras::install_image_loaders(&cc.egui_ctx);
		let st = Settings::load("Crossbell-SaveEditor.ini", "Crossbell-SaveEditor");
		let pal = Pal::get(st.get("dark") != Some("0"));
		apply_visuals(&cc.egui_ctx, &pal);
		let mut app = App {
			pal,
			dir: String::new(),
			st,
			slots: Vec::new(),
			sel: None,
			data: None,
			orig: Vec::new(),
			tab: 0,
			item_tab: 0,
			lsel: None,
			drag_src: None,
			chr_sel: 0,
			pick_game: None,
			search: String::new(),
			owned_only: true,
			collapsed: std::collections::HashSet::new(),
			game: None,
			g: &save::ZERO,
			items: Vec::new(),
			item_of: HashMap::new(),
			names: HashMap::new(),
			crafts: HashMap::new(),
			orbs: HashMap::new(),
			mods: HashMap::new(),
			slot_costs: HashMap::new(),
			faces: HashMap::new(),
			face_tex: HashMap::new(),
			art_tex: HashMap::new(),
			icons: None,
			status: "Select a slot on the left. Before writing, the program backs the slot up to the «Backups» folder next to it.".into(),
			dialog: Dialog::None,
			zoom: 1.0,
			about: false,
			tips: false,
			allow_close: false,
			scan_at: 0.0,
			scan_sig: 0,
			scan_dir: String::new(),
			gsearch: String::new(),
			gsearch_open: false,
			scroll_sel: false,
			shown_title: String::new(),
		};
		app.zoom = app.st.get("zoom").and_then(|v| v.parse().ok()).unwrap_or(1.0f32).clamp(0.7, 1.6);
		cc.egui_ctx.set_zoom_factor(app.zoom);
		app.tips = app.st.get("tips_seen") != Some("1");
		app.tab = match app.st.get("tab3").and_then(|v| v.parse::<usize>().ok()) {
			Some(v) => v.min(2),
			None => match app.st.get("tab").and_then(|v| v.parse::<usize>().ok()).unwrap_or(0) {
				3 => 2,
				2 => 1,
				v => v.min(1),
			},
		};
		app.pick_game = app.st.get("last_game").and_then(|v| v.parse::<usize>().ok()).filter(|&i| i < GAMES.len());
		app
	}

	fn key(&self, k: &str) -> String {
		format!("{k}_{}", GAMES[self.game.unwrap_or(0)].0)
	}

	fn table(&self, f: &str) -> &'static [u8] {
		kiseki_tool::pack::get(&format!("{}/{f}", self.g.dir)).unwrap_or(&[])
	}

	fn set_game(&mut self, i: usize) {
		self.save_settings();
		self.game = Some(i);
		self.st.set("last_game", &i.to_string());
		self.g = profile(i);
		let azure = self.g.master_quartz;
		self.items = save::items(&[self.table("t_item._dt"), self.table("t_item2._dt")], &[self.table("t_ittxt._dt"), self.table("t_ittxt2._dt")], self.table("t_quartz._dt"), azure);
		self.item_of = self.items.iter().enumerate().map(|(k, it)| (it.id, k)).collect();
		self.names = save::names(self.table("t_name._dt"));
		self.crafts = save::crafts_table(self.table("t_magic._dt"));
		self.orbs = save::orb_layouts(self.table("t_orb._dt"));
		self.mods = save::mods_table(&self.items);
		self.slot_costs = save::slot_costs(self.table("t_sltget._dt"));
		self.faces = kiseki_tool::kai::save::faces(self.table("faces.bin"));
		self.face_tex.clear();
		self.icons = None;
		self.data = None;
		self.sel = None;
		self.chr_sel = 0;
		self.lsel = None;
		self.drag_src = None;
		self.dir = self.st.get(&self.key("dir")).map(|s| s.to_string()).filter(|s| !s.is_empty()).unwrap_or_else(|| default_dir(i));
		self.item_tab = self.st.get(&self.key("item_tab")).and_then(|v| v.parse().ok()).unwrap_or(0).min(save::TABS.len() - 1);
		if self.items.is_empty() {
			self.status = format!("✖ No {} tables in data.dat next to the program — item names can't be shown", self.g.dir);
		}
		self.rescan();
		if let Some(k) = self.st.get(&self.key("slot")).and_then(|n| self.slots.iter().position(|s| s.name == n)) {
			self.open(k);
		}
	}

	fn save_settings(&mut self) {
		self.st.set("dark", if self.pal.dark { "1" } else { "0" });
		self.st.set("tab3", &self.tab.to_string());
		self.st.set("zoom", &format!("{:.2}", self.zoom));
		if self.game.is_some() {
			self.st.set(&self.key("dir"), &self.dir);
			self.st.set(&self.key("item_tab"), &self.item_tab.to_string());
			let slot = self.sel.and_then(|i| self.slots.get(i)).map(|s| s.name.clone()).unwrap_or_default();
			self.st.set(&self.key("slot"), &slot);
		}
		self.st.save();
	}

	fn rescan(&mut self) {
		let keep = self.sel.and_then(|i| self.slots.get(i)).map(|s| s.name.clone());
		self.slots = read_slots(Path::new(self.dir.trim()));
		self.sel = keep.and_then(|n| self.slots.iter().position(|s| s.name == n));
	}

	fn open(&mut self, i: usize) {
		self.sel = Some(i);
		self.scroll_sel = true;
		match save::read_for(self.g, &self.slots[i].dir.join("savedata.dat")) {
			Ok(d) => {
				self.status = if save::sums_ok(&d) {
					format!("Opened {}: {} — {}", self.slots[i].name, self.slots[i].title, self.slots[i].sub)
				} else {
					format!("Opened {}: the checksums don't match (the slot was edited by another tool) — they will be recalculated on write", self.slots[i].name)
				};
				self.orig = d.clone();
				self.data = Some(d);
			}
			Err(e) => {
				self.data = None;
				self.wrong_game(&e);
				self.status = format!("✖ {e}");
			}
		}
	}

	fn wrong_game(&mut self, e: &str) {
		if let (Some((_, other)), Some(i)) = (e.rsplit_once(": this save is from "), self.game) {
			self.dialog = Dialog::Info(format!("This is a save from {other}, but the selected game is {}.\n\nChoose the right game (“Select game”) or another save folder.", GAMES[i].1));
		}
	}

	fn open_file(&mut self, file: &Path) {
		let (Some(slot), Some(root)) = (file.parent(), file.parent().and_then(|p| p.parent())) else { return };
		self.dir = root.display().to_string();
		self.rescan();
		match self.slots.iter().position(|s| s.dir == slot) {
			Some(i) => self.open(i),
			None => match save::read_for(self.g, file) {
				Ok(d) => {
					self.slots.push(Slot { name: slot.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(), dir: slot.to_path_buf(), title: "(opened file)".into(), sub: file.display().to_string(), when: String::new(), playtime: String::new(), difficulty: String::new() });
					self.sel = Some(self.slots.len() - 1);
					self.orig = d.clone();
					self.data = Some(d);
					self.status = format!("Opened {}", file.display());
				}
				Err(e) => {
					self.wrong_game(&e);
					self.status = format!("✖ {e}");
				}
			},
		}
		self.save_settings();
	}

	fn texture(ctx: &egui::Context, name: &str, png: &[u8]) -> Option<egui::TextureHandle> {
		image::load_from_memory(png).ok().map(|img| {
			let img = img.to_rgba8();
			ctx.load_texture(name, egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw()), egui::TextureOptions::LINEAR)
		})
	}

	fn face(&mut self, ctx: &egui::Context, who: u32) -> Option<egui::TextureHandle> {
		if let Some(t) = self.face_tex.get(&who) {
			return t.clone();
		}
		let t = self.faces.get(&who).and_then(|png| Self::texture(ctx, &format!("face{who}"), png));
		self.face_tex.insert(who, t.clone());
		t
	}

	fn cover(&mut self, ctx: &egui::Context, key: &str) -> Option<egui::TextureHandle> {
		if let Some(t) = self.art_tex.get(key) {
			return t.clone();
		}
		let t = kiseki_tool::pack::get(&format!("covers/{key}.png")).and_then(|png| Self::texture(ctx, key, png));
		self.art_tex.insert(key.to_string(), t.clone());
		t
	}

	fn icons(&mut self, ctx: &egui::Context) -> Option<egui::TextureHandle> {
		let png = self.table("c_icon1.png");
		self.icons.get_or_insert_with(|| Self::texture(ctx, "icons", png)).clone()
	}

	fn changed(&self) -> bool {
		self.data.as_ref().is_some_and(|d| *d != self.orig)
	}

	fn ask_open(&mut self, i: usize) {
		if self.sel == Some(i) && self.data.is_some() {
			return;
		}
		if self.changed() {
			self.dialog = Dialog::Ask("There are unsaved changes. Open another slot without saving?".into(), Act::Open(i));
		} else {
			self.open(i);
		}
	}

	fn overwrite(&mut self) {
		let (Some(i), Some(mut d)) = (self.sel, self.data.clone()) else { return };
		let slot = &self.slots[i];
		let backup = exe_dir().join("Backups").join(GAMES[self.game.unwrap_or(0)].1).join(format!("{}_{}", slot.name, stamp()));
		if let Err(e) = copy_dir(&slot.dir, &backup) {
			self.dialog = Dialog::Info(format!("Could not back up the slot — it was not changed.\n{e}"));
			return;
		}
		match save::write(&slot.dir.join("savedata.dat"), &mut d) {
			Ok(()) => {
				self.orig = d.clone();
				self.data = Some(d);
				self.status = format!("Written to {}. Backup of the previous slot: {}", slot.name, backup.display());
			}
			Err(e) => self.dialog = Dialog::Info(format!("Could not write {}:\n{e}\n\nThe slot was not changed (backup: {}).", slot.name, backup.display())),
		}
	}

	fn save_new(&mut self) {
		let (Some(i), Some(mut d)) = (self.sel, self.data.clone()) else { return };
		let root = PathBuf::from(self.dir.trim());
		let Some(n) = (1..200).map(|n| format!("data{n:04}")).find(|n| !root.join(n).exists()) else { return };
		let to = root.join(&n);
		let from = self.slots[i].dir.clone();
		let res = (|| -> Result<(), String> {
			std::fs::create_dir_all(&to).map_err(|e| e.to_string())?;
			for f in ["info.txt", "icon0.png", "messagelog.dat"] {
				if from.join(f).is_file() {
					std::fs::copy(from.join(f), to.join(f)).map_err(|e| e.to_string())?;
				}
			}
			save::write(&to.join("savedata.dat"), &mut d)
		})();
		match res {
			Ok(()) => {
				self.status = format!("Written to new slot {n} (in game it looks like {}).", self.slots[i].name);
				self.rescan();
				if let Some(k) = self.slots.iter().position(|s| s.name == n) {
					self.open(k);
				}
			}
			Err(e) => {
				let _ = std::fs::remove_dir_all(&to);
				self.dialog = Dialog::Info(format!("Could not write {n}:\n{e}"));
			}
		}
	}

	fn who_name(&self, who: u32) -> String {
		short_name(&self.names, who)
	}
	fn party_name_id(&self, d: &[u8], p: u32) -> u32 {
		if (p as usize) < save::CHARS { save::record_name(self.g, d, p as usize) } else { p }
	}

	fn picker(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
		let p = self.pal;
		let full = ui.max_rect();
		let covers: Vec<Option<egui::TextureHandle>> = GAMES.iter().map(|g| self.cover(ctx, g.0)).collect();
		const ACCENT: [Color32; 2] = [Color32::from_rgb(0xE0, 0x8A, 0x3C), Color32::from_rgb(0x4A, 0x8F, 0xE7)];
		const ARC: [&str; 2] = ["CROSSBELL ARC · I", "CROSSBELL ARC · II"];
		let mut open = None;
		let head = Rect::from_min_size(full.min, Vec2::new(full.width(), 56.0));
		ui.painter().text(Pos2::new(head.min.x + 24.0, head.center().y), Align2::LEFT_CENTER, title(), f_semi(15.0), p.text);
		{
			let mut r = ui.new_child(egui::UiBuilder::new().max_rect(head.shrink2(Vec2::new(24.0, 14.0))).layout(egui::Layout::right_to_left(egui::Align::Center)));
			r.spacing_mut().item_spacing.x = 8.0;
			if theme_button(&mut r, &p, 28.0).clicked() {
				self.pal = Pal::get(!self.pal.dark);
				apply_visuals(ctx, &self.pal);
				self.save_settings();
			}
		}
		ui.painter().line_segment([Pos2::new(full.min.x, head.max.y), Pos2::new(full.max.x, head.max.y)], Stroke::new(1.0, p.div));
		let foot = Rect::from_min_max(Pos2::new(full.min.x, full.max.y - 72.0), full.max);
		ui.painter().rect_filled(foot, 0, p.surface);
		ui.painter().line_segment([foot.left_top(), foot.right_top()], Stroke::new(1.0, p.div));
		let cw = (full.width() - 64.0).min(960.0);
		let x0 = full.center().x - cw / 2.0;
		let gap = 24.0;
		let cols = if cw >= 2.0 * 280.0 + gap { 2 } else { 1 };
		let card_w = (cw - gap * (cols - 1) as f32) / cols as f32;
		let rows = GAMES.len().div_ceil(cols);
		let fit_h = (foot.min.y - head.max.y - 84.0 - 52.0 - (rows - 1) as f32 * gap) / rows as f32 - 116.0;
		let art_h = (card_w * 430.0 / 920.0).min(fit_h.max(120.0));
		let card_h = art_h + 116.0;
		let block_h = 84.0 + rows as f32 * card_h + (rows - 1) as f32 * gap;
		let mut y = head.max.y + ((foot.min.y - head.max.y - block_h) / 2.0).max(40.0);
		ui.painter().text(Pos2::new(x0, y), Align2::LEFT_TOP, "Select a game", f_semi(26.0), p.text);
		y += 40.0;
		ui.painter().text(Pos2::new(x0, y), Align2::LEFT_TOP, "Then pick a save slot. The editor backs up the slot before writing.", f_body(14.0), p.muted);
		y += 44.0;
		let area = Rect::from_min_max(Pos2::new(full.min.x, y), Pos2::new(full.max.x, foot.min.y - 12.0));
		let mut c = ui.new_child(egui::UiBuilder::new().max_rect(area));
		egui::ScrollArea::vertical().id_salt("picker").auto_shrink([false, false]).show(&mut c, |ui| {
			let top = ui.cursor().min.y;
			ui.allocate_space(Vec2::new(area.width(), rows as f32 * (card_h + gap)));
			for (i, g) in GAMES.iter().enumerate() {
				let r = Rect::from_min_size(Pos2::new(x0 + (i % cols) as f32 * (card_w + gap), top + (i / cols) as f32 * (card_h + gap)), Vec2::new(card_w, card_h));
				let resp = ui.interact(r, ui.id().with(("game", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
				let on = self.pick_game == Some(i);
				let pt = ui.painter();
				pt.rect_filled(r, 12, if resp.hovered() { p.div2 } else { p.surface });
				let ar = Rect::from_min_size(r.min, Vec2::new(card_w, art_h));
				match &covers[i] {
					Some(t) => {
						let ts = t.size_vec2();
						let (want, have) = (card_w / art_h, ts.x / ts.y.max(1.0));
						let uv = if have < want {
							let h = have / want;
							let y0 = ((1.0 - h) * 0.5).clamp(0.0, 1.0 - h);
							Rect::from_min_max(Pos2::new(0.0, y0), Pos2::new(1.0, y0 + h))
						} else {
							let w = want / have;
							Rect::from_min_max(Pos2::new((1.0 - w) / 2.0, 0.0), Pos2::new((1.0 + w) / 2.0, 1.0))
						};
						egui::Image::new(t).uv(uv).corner_radius(egui::CornerRadius { nw: 12, ne: 12, sw: 0, se: 0 }).paint_at(ui, ar);
					}
					None => {
						pt.rect_filled(ar, 12, p.bg);
					}
				}
				let tx = r.min.x + 20.0;
				pt.text(Pos2::new(tx, ar.max.y + 18.0), Align2::LEFT_TOP, ARC[i], f_semi(11.0), ACCENT[i]);
				pt.text(Pos2::new(tx, ar.max.y + 36.0), Align2::LEFT_TOP, g.1, f_semi(19.0), p.text);
				let pill_t = if on { "Selected" } else { "Open saves" };
				let pw = ui.fonts(|f| f.layout_no_wrap(pill_t.to_string(), f_body(12.5), p.text).size().x) + 28.0;
				let pr = Rect::from_min_size(Pos2::new(tx, r.max.y - 38.0), Vec2::new(pw, 26.0));
				pt.rect(pr, 13, if on { p.sel } else { p.bg }, Stroke::new(1.0, if on { ACCENT[i] } else { p.div }), StrokeKind::Inside);
				pt.text(pr.center(), Align2::CENTER_CENTER, pill_t, f_body(12.5), p.text);
				let stroke = if on { Stroke::new(2.0, ACCENT[i]) } else { Stroke::new(1.0, p.div) };
				pt.rect_stroke(r, 12, stroke, StrokeKind::Inside);
				if resp.double_clicked() {
					open = Some(i);
				} else if resp.clicked() {
					self.pick_game = if on { None } else { Some(i) };
				}
			}
		});
		match self.pick_game {
			Some(i) => {
				ui.painter().text(Pos2::new(x0, foot.center().y - 9.0), Align2::LEFT_CENTER, "Selected", f_body(11.5), p.muted);
				ui.painter().text(Pos2::new(x0, foot.center().y + 9.0), Align2::LEFT_CENTER, GAMES[i].1, f_semi(15.0), p.text);
			}
			None => {
				ui.painter().text(Pos2::new(x0, foot.center().y), Align2::LEFT_CENTER, "Back up your save files before editing. This is an unofficial fan tool.", f_body(13.0), p.muted);
			}
		}
		let br = Rect::from_min_max(Pos2::new(x0, foot.min.y + 14.0), Pos2::new(x0 + cw, foot.max.y - 14.0));
		let mut b = ui.new_child(egui::UiBuilder::new().max_rect(br).layout(egui::Layout::right_to_left(egui::Align::Center)));
		b.spacing_mut().item_spacing.x = 10.0;
		let sel = self.pick_game;
		if primary_button_icon(&mut b, &p, "Continue", Some("arrow-right"), Vec2::new(150.0, 40.0), 9, f_semi(15.0), sel.is_some()).clicked() {
			open = sel;
		}
		if small_button(&mut b, &p, "Browse save folder…", Some("folder-open"), 40.0, sel.is_some(), p.text).on_hover_text("Choose the save folder of the selected game").clicked() {
			if let Some(i) = sel {
				if let Some(dir) = pick_folder(&default_dir(i)) {
					self.set_game(i);
					self.dir = dir.display().to_string();
					self.data = None;
					self.sel = None;
					self.rescan();
					self.save_settings();
					self.shown_title.clear();
				}
			}
		}
		if let Some(i) = open {
			self.pick_game = Some(i);
			self.set_game(i);
			self.shown_title.clear();
		}
	}

	fn search_results(&mut self, ctx: &egui::Context, at: Rect, q: &str) {
		let p = self.pal;
		let g = self.g;
		let Some(d) = self.data.as_ref() else { return };
		let mut hits: Vec<(String, String, usize, u32)> = Vec::new();
		for &k in g.playable {
			if save::u16_at(d, save::chr_at(g, k) + field::LEVEL) == 0 {
				continue;
			}
			let who = self.who_name(save::record_name(g, d, k));
			if who.to_lowercase().contains(q) {
				hits.push(("Character".into(), who.clone(), 1, k as u32));
			}
			for c in save::scrafts(g, d, k).into_iter().chain(save::crafts(g, d, k)) {
				if let Some(cr) = self.crafts.get(&c).filter(|cr| cr.name.to_lowercase().contains(q)) {
					hits.push((format!("Craft · {who}"), cr.name.clone(), 1, k as u32));
				}
			}
		}
		for it in self.items.iter().filter(|it| it.name.to_lowercase().contains(q)) {
			let n = save::item_count(g, d, it.id);
			let tab = save::tab_of(it).map_or("", |t| save::TABS[t].0);
			hits.push((format!("{tab} · {n}"), it.name.clone(), 0, it.id));
		}
		let mut go = None;
		let area = egui::Area::new(egui::Id::new("gsearch_pop")).order(egui::Order::Foreground).fixed_pos(at.left_bottom() + Vec2::new(0.0, 4.0)).show(ctx, |ui| {
			egui::Frame::popup(ui.style()).show(ui, |ui| {
				ui.set_width(at.width().max(320.0));
				if hits.is_empty() {
					ui.label(egui::RichText::new("Nothing found").font(f_body(13.0)).color(p.muted));
				}
				egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
					for (i, (kind, name, to, id)) in hits.iter().enumerate().take(60) {
						let r = ui.add(egui::Button::new(egui::RichText::new(format!("{name}   ")).font(f_body(13.0)).color(p.text)).shortcut_text(egui::RichText::new(kind).font(f_body(11.5)).color(p.muted)).frame(false).min_size(Vec2::new(ui.available_width(), 24.0)));
						if r.clicked() {
							go = Some(i);
						}
						let _ = (to, id);
					}
				});
			});
		});
		let outside = ctx.input(|i| i.pointer.any_click()) && !area.response.rect.contains(ctx.input(|i| i.pointer.interact_pos().unwrap_or_default())) && !at.contains(ctx.input(|i| i.pointer.interact_pos().unwrap_or_default()));
		if ctx.input(|i| i.key_pressed(egui::Key::Escape)) || outside {
			self.gsearch_open = false;
		}
		if let Some(i) = go {
			let (_, name, to, id) = hits[i].clone();
			self.gsearch_open = false;
			if to == 0 {
				if let Some(it) = self.item_of.get(&id).map(|&k| &self.items[k]) {
					self.item_tab = save::tab_of(it).unwrap_or(0);
				}
				self.tab = 2;
				self.search = name;
				self.owned_only = false;
			} else {
				self.tab = 1;
				self.chr_sel = id as usize;
			}
		}
	}

	fn windows(&mut self, ctx: &egui::Context) {
		let p = self.pal;
		if self.about {
			let backups = exe_dir().join("Backups");
			let mut open = true;
			egui::Window::new("About").open(&mut open).collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, Vec2::ZERO).show(ctx, |ui| {
				ui.label(egui::RichText::new(title()).font(f_semi(16.0)).color(p.text));
				ui.label(egui::RichText::new("Save editor for Trails from Zero and Trails to Azure (PC).").font(f_body(13.0)).color(p.muted));
				ui.add_space(8.0);
				ui.label(egui::RichText::new("Slot backups:").font(f_body(12.5)).color(p.muted));
				ui.label(egui::RichText::new(backups.display().to_string()).font(f_mono(12.0)).color(p.text));
				ui.add_space(8.0);
				ui.horizontal(|ui| {
					if small_button(ui, &p, "Open backups folder", Some("folder-open"), 28.0, true, p.text).clicked() {
						let _ = std::fs::create_dir_all(&backups);
						open_folder(&backups.display().to_string());
					}
					if small_button(ui, &p, "Program folder", Some("folder-open"), 28.0, true, p.text).clicked() {
						open_folder(&exe_dir().display().to_string());
					}
				});
				ui.add_space(6.0);
				ui.label(egui::RichText::new("Unofficial fan tool. Keep backups of your saves.").font(f_body(12.0)).color(p.muted));
			});
			self.about = open;
		}
		if self.tips && self.game.is_some() {
			if self.st.get("tips_seen") != Some("1") {
				self.st.set("tips_seen", "1");
				self.st.save();
			}
			let mut close = false;
			egui::Window::new("Tips").collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, Vec2::ZERO).show(ctx, |ui| {
				ui.set_max_width(460.0);
				for t in [
					"Save slots are on the left; Up / Down arrows — next slot. You can drop a savedata.dat file onto the window.",
					"«General» — mira, DP and detective rank, sepith, combat notebook, play time, difficulty.",
					"«Party» — drag a card onto another one, or click two cards, to swap them; drag into «Can be added» to remove from the party.",
					"«Items» — the amount of any item; untick «Owned only» to add a new one.",
					"Before writing, the slot is copied to «Backups» next to the program (About opens the folder).",
					"Ctrl+S — save to this slot, Ctrl+F — search across all tabs, Ctrl + mouse wheel — zoom.",
					"Numbers: drag left/right, or click and type.",
				] {
					ui.label(egui::RichText::new(format!("•  {t}")).font(f_body(13.0)).color(p.text));
					ui.add_space(3.0);
				}
				ui.add_space(6.0);
				if primary_button_icon(ui, &p, "Got it", Some("check"), Vec2::new(120.0, 30.0), 8, f_semi(14.0), true).clicked() {
					close = true;
				}
			});
			if close {
				self.tips = false;
				self.st.set("tips_seen", "1");
				self.st.save();
			}
		}
	}

	fn global_input(&mut self, ctx: &egui::Context) {
		let z = ctx.input(|i| i.zoom_delta());
		if z != 1.0 {
			self.zoom = (self.zoom * z).clamp(0.7, 1.6);
			ctx.set_zoom_factor(self.zoom);
			self.save_settings();
		}
		if let Some(path) = ctx.input(|i| i.raw.dropped_files.iter().find_map(|f| f.path.clone())) {
			let game = save::read(&path).ok().map(|d| usize::from(d.len() == save::AZURE.size));
			match game {
				Some(gi) => {
					if self.game != Some(gi) {
						self.pick_game = Some(gi);
						self.set_game(gi);
						self.shown_title.clear();
					}
					self.open_file(&path);
				}
				None => self.dialog = Dialog::Info(format!("{} is not a Zero / Azure save.", path.display())),
			}
		}
		if ctx.input(|i| i.viewport().close_requested()) && self.changed() && !self.allow_close {
			ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
			self.dialog = Dialog::Ask("There are unsaved changes. Close without saving?".into(), Act::Close);
		}
		let now = ctx.input(|i| i.time);
		if self.game.is_some() && now - self.scan_at > 2.0 {
			self.scan_at = now;
			let sig = dir_signature(Path::new(self.dir.trim()));
			if self.scan_dir != self.dir.trim() {
				self.scan_dir = self.dir.trim().to_string();
				self.scan_sig = 0;
			}
			if self.scan_sig != 0 && sig != self.scan_sig {
				self.rescan();
				self.status = match (self.sel.and_then(|i| self.slots.get(i)), self.data.is_some()) {
					(Some(s), _) => format!("Slot list updated · open: {}", s.name),
					(None, true) => "Slot list updated · the open slot is no longer in the folder (save to a new slot)".into(),
					(None, false) => "Slot list updated — the save folder changed".into(),
				};
				self.scroll_sel = true;
			}
			self.scan_sig = sig;
		}
		ctx.request_repaint_after(std::time::Duration::from_secs(2));
		if self.game.is_none() || !matches!(self.dialog, Dialog::None) {
			return;
		}
		let typing = ctx.memory(|m| m.focused().is_some());
		let (ctrl_s, ctrl_f, up, down) = ctx.input(|i| (i.modifiers.command && i.key_pressed(egui::Key::S), i.modifiers.command && i.key_pressed(egui::Key::F), i.key_pressed(egui::Key::ArrowUp), i.key_pressed(egui::Key::ArrowDown)));
		if ctrl_s && self.changed() {
			let name = self.sel.map(|i| self.slots[i].name.clone()).unwrap_or_default();
			self.dialog = Dialog::Ask(format!("Write changes to {name}?\n\nThe previous slot is first copied to the «Backups» folder next to the program."), Act::Overwrite);
		}
		if ctrl_f && self.data.is_some() {
			ctx.memory_mut(|m| m.request_focus(egui::Id::new("gsearch")));
		}
		if typing {
			return;
		}
		if (up || down) && !self.slots.is_empty() {
			let n = self.slots.len();
			let i = match self.sel {
				Some(i) if up => i.saturating_sub(1),
				Some(i) => (i + 1).min(n - 1),
				None => 0,
			};
			self.ask_open(i);
		}
	}

	fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
		let p = self.pal;
		let full = ui.max_rect();
		let x0 = full.min.x + 20.0;
		let w = full.width() - 40.0;
		let mut y = full.min.y + 16.0;
		ui.painter().galley(Pos2::new(x0, y), ui.fonts(|f| f.layout_job(caps_job("Save folder", p.muted))), p.muted);
		{
			let mut r = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(Pos2::new(x0, y + 21.0), Vec2::new(w, 36.0))));
			let hint = format!("C:\\Users\\…\\Saved Games\\Falcom\\{}", self.g.falcom);
			let (ch, cl, _) = path_field(&mut r, &p, "dir", &mut self.dir, 36.0, &hint, "Browse…", false, true);
			let br = Rect::from_min_size(Pos2::new(x0 + w - 360.0, y - 4.0), Vec2::new(360.0, 22.0));
			let mut b = ui.new_child(egui::UiBuilder::new().max_rect(br).layout(egui::Layout::right_to_left(egui::Align::Center)));
			if small_button(&mut b, &p, "Open save…", Some("file"), 22.0, true, p.accent).on_hover_text("Pick savedata.dat from any dataNNNN folder").clicked() {
				if let Some(file) = pick_files(&self.dir, "Open save", "Zero / Azure save", &["dat"], false).into_iter().next() {
					self.open_file(&file);
				}
			}
			b.add_space(8.0);
			if small_button(&mut b, &p, "Select game", Some("switch"), 22.0, true, p.muted).on_hover_text("Back to game selection").clicked() {
				self.save_settings();
				self.game = None;
				self.status.clear();
				self.shown_title.clear();
			}
			if cl {
				if let Some(d) = pick_folder(&self.dir) {
					self.dir = d.display().to_string();
				}
			}
			if ch || cl {
				self.data = None;
				self.sel = None;
				self.rescan();
				self.save_settings();
			}
		}
		y += 21.0 + 36.0 + 8.0;
		{
			let mut r = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, 28.0))).layout(egui::Layout::right_to_left(egui::Align::Center)));
			r.spacing_mut().item_spacing.x = 8.0;
			if theme_button(&mut r, &p, 26.0).clicked() {
				self.pal = Pal::get(!self.pal.dark);
				apply_visuals(ctx, &self.pal);
				self.save_settings();
			}
			if small_button(&mut r, &p, "About", Some("info"), 26.0, true, p.muted).clicked() {
				self.about = true;
			}
			if small_button(&mut r, &p, "Tips", Some("help"), 26.0, true, p.muted).clicked() {
				self.tips = true;
			}
			if self.data.is_some() {
				r.add_space(12.0);
				let resp = r.add(egui::TextEdit::singleline(&mut self.gsearch).id(egui::Id::new("gsearch")).hint_text("Search everywhere (Ctrl+F)").desired_width(260.0));
				if resp.changed() || resp.gained_focus() {
					self.gsearch_open = true;
				}
				let q = self.gsearch.trim().to_lowercase();
				if self.gsearch_open && q.chars().count() >= 2 {
					self.search_results(ctx, resp.rect, &q);
				}
			}
		}
		y += 36.0;
		let bottom = full.max.y - 56.0;
		let lw = 300.0;
		let list = Rect::from_min_max(Pos2::new(x0, y), Pos2::new(x0 + lw, bottom));
		ui.painter().rect(list, 10, Color32::TRANSPARENT, Stroke::new(1.0, p.div), StrokeKind::Inside);
		let mut open = None;
		{
			let mut c = ui.new_child(egui::UiBuilder::new().max_rect(list.shrink(6.0)));
			let mut area = egui::ScrollArea::vertical().id_salt(("slots", self.g.dir)).auto_shrink([false, false]);
			if std::mem::take(&mut self.scroll_sel) {
				if let Some(i) = self.sel {
					let row = 82.0 + c.spacing().item_spacing.y;
					let view = c.available_height();
					let cur = egui::scroll_area::State::load(c.ctx(), c.make_persistent_id(egui::Id::new(("slots", self.g.dir)))).map_or(0.0, |s| s.offset.y);
					let top = i as f32 * row;
					if top < cur || top + 82.0 > cur + view {
						area = area.vertical_scroll_offset(if top < cur { top } else { top + 82.0 - view });
					}
				}
			}
			area.show_rows(&mut c, 82.0, self.slots.len(), |ui, range| {
				for i in range {
					let s = &self.slots[i];
					let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 82.0), Sense::click());
					if self.sel == Some(i) {
						ui.painter().rect_filled(r, 7, p.sel);
					} else if resp.hovered() {
						ui.painter().rect_filled(r, 7, p.div2.gamma_multiply(0.6));
					}
					ui.painter().text(Pos2::new(r.min.x + 10.0, r.min.y + 13.0), Align2::LEFT_CENTER, slot_label(&s.name), f_mono(12.0), p.muted);
					ui.painter().text(Pos2::new(r.max.x - 10.0, r.min.y + 13.0), Align2::RIGHT_CENTER, &s.when, f_body(11.5), p.muted);
					let w = r.width() - 20.0;
					paint_text_clipped(ui, Rect::from_min_size(Pos2::new(r.min.x + 10.0, r.min.y + 24.0), Vec2::new(w, 18.0)), &s.title, f_semi(13.5), p.text, Align2::LEFT_CENTER);
					paint_text_clipped(ui, Rect::from_min_size(Pos2::new(r.min.x + 10.0, r.min.y + 43.0), Vec2::new(w, 17.0)), &s.sub, f_body(12.5), p.text, Align2::LEFT_CENTER);
					let mut pt = if s.playtime.is_empty() { String::new() } else { format!("Play time {}", s.playtime) };
					if !s.difficulty.is_empty() {
						pt += &format!("  ·  {}", s.difficulty);
					}
					paint_text_clipped(ui, Rect::from_min_size(Pos2::new(r.min.x + 10.0, r.min.y + 62.0), Vec2::new(w, 16.0)), &pt, f_body(11.5), p.muted, Align2::LEFT_CENTER);
					if resp.clicked() {
						open = Some(i);
					}
				}
			});
			if self.slots.is_empty() {
				ui.painter().text(list.center(), Align2::CENTER_CENTER, "No saves found", f_body(13.0), p.muted);
			}
		}
		if let Some(i) = open {
			self.ask_open(i);
		}
		let right = Rect::from_min_max(Pos2::new(x0 + lw + 16.0, y), Pos2::new(x0 + w, bottom));
		if self.data.is_none() {
			ui.painter().rect(right, 10, Color32::TRANSPARENT, Stroke::new(1.0, p.div), StrokeKind::Inside);
			ui.painter().text(right.center(), Align2::CENTER_CENTER, "Select a slot on the left", f_body(14.0), p.muted);
		} else {
			let mut r = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(right.min, Vec2::new(right.width(), 36.0))));
			segmented(&mut r, &p, "tab", &["General", "Party", "Items"], &mut self.tab, None, 36.0, 9, f_body(14.0), &[]);
			let body = Rect::from_min_max(Pos2::new(right.min.x, right.min.y + 48.0), right.max);
			ui.painter().rect(body, 10, Color32::TRANSPARENT, Stroke::new(1.0, p.div), StrokeKind::Inside);
			let mut c = ui.new_child(egui::UiBuilder::new().max_rect(body.shrink2(Vec2::new(16.0, 12.0))));
			match self.tab {
				0 => self.general(&mut c, ctx),
				1 => self.lineup(&mut c, ctx),
				_ => self.inventory(&mut c, ctx),
			}
		}
		let br = Rect::from_min_max(Pos2::new(x0, bottom + 12.0), Pos2::new(x0 + w, full.max.y - 10.0));
		let mut r = ui.new_child(egui::UiBuilder::new().max_rect(br).layout(egui::Layout::right_to_left(egui::Align::Center)));
		r.spacing_mut().item_spacing.x = 10.0;
		let can = self.changed();
		if small_button(&mut r, &p, "Save to new slot", Some("file-plus"), 34.0, self.data.is_some(), p.text).on_hover_text("Writes the edited save to the first free slot; the selected one stays as it was").clicked() {
			self.save_new();
		}
		if primary_button_icon(&mut r, &p, "Save to this slot", Some("save"), Vec2::new(220.0, 34.0), 9, f_semi(15.0), can).clicked() {
			let name = self.sel.map(|i| self.slots[i].name.clone()).unwrap_or_default();
			self.dialog = Dialog::Ask(format!("Write changes to {name}?\n\nThe previous slot is first copied to the «Backups» folder next to the program."), Act::Overwrite);
		}
		if can && small_button(&mut r, &p, "Discard changes", Some("undo"), 34.0, true, p.muted).clicked() {
			self.data = Some(self.orig.clone());
		}
		let sw = r.available_width();
		paint_text_clipped(&r, Rect::from_min_size(Pos2::new(br.min.x, br.center().y - 9.0), Vec2::new(sw - 10.0, 18.0)), &self.status, f_body(12.5), if self.status.starts_with('✖') { p.warn } else { p.muted }, Align2::LEFT_CENTER);
	}

	fn general(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
		let p = self.pal;
		let atlas = self.icons(ctx);
		let (chapter, place) = self.sel.and_then(|i| self.slots.get(i)).map(|s| (s.title.clone(), s.sub.clone())).unwrap_or_default();
		let g = self.g;
		let d = self.data.as_mut().unwrap();
		const LW: f32 = 118.0;
		const RW: f32 = 196.0;
		const FW: f32 = 96.0;
		const GAP: f32 = 10.0;
		const LEFT_W: f32 = 2.0 * (LW + GAP + FW) + GAP;
		const RIGHT_W: f32 = 2.0 * (RW + GAP + FW) + GAP;
		const COL_GAP: f32 = 48.0;
		let head = |ui: &mut egui::Ui, t: &str| {
			ui.add_space(14.0);
			ui.label(egui::RichText::new(t).font(f_semi(15.5)).color(p.text));
			ui.add_space(4.0);
		};
		let label = |ui: &mut egui::Ui, w: f32, t: &str| {
			ui.allocate_ui_with_layout(Vec2::new(w, 26.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
				ui.set_min_width(w);
				ui.label(egui::RichText::new(t).font(f_body(13.0)).color(p.muted));
			});
		};
		let num = |ui: &mut egui::Ui, v: &mut u32, max: u32, speed: f64| ui.add_sized([FW, 26.0], egui::DragValue::new(v).clamp_existing_to_range(false).range(0..=max).speed(speed)).on_hover_text(num_hint()).changed();
		let wide = ui.available_width() >= LEFT_W + COL_GAP + RIGHT_W;
		let row_layout = if wide { egui::Layout::left_to_right(egui::Align::Min) } else { egui::Layout::top_down(egui::Align::Min) };
		egui::ScrollArea::vertical().id_salt("general").auto_shrink([false, false]).show(ui, |ui| {
			ui.spacing_mut().item_spacing = Vec2::new(GAP, 6.0);
			ui.add_space(4.0);
			ui.add(egui::Label::new(egui::RichText::new(if chapter.is_empty() { "—" } else { chapter.as_str() }).font(f_semi(26.0)).color(p.text)).wrap());
			if !place.is_empty() {
				ui.label(egui::RichText::new(&place).font(f_semi(14.0)).color(p.muted));
			}
			head(ui, "Mira");
			ui.horizontal(|ui| {
				label(ui, LW, "Mira");
				let mut mira = save::u32_at(d, g.mira);
				if num(ui, &mut mira, 9_999_999, 100.0) {
					save::set_u32(d, g.mira, mira);
				}
			});
			ui.with_layout(row_layout, |ui| {
				ui.vertical(|ui| {
					ui.set_width(LEFT_W);
					head(ui, "Detective Notebook");
					ui.horizontal(|ui| {
						label(ui, LW, "DP");
						let mut dp = save::u32_at(d, g.dp);
						if num(ui, &mut dp, 99_999, 1.0) {
							save::set_u32(d, g.dp, dp);
						}
						if !g.ranks.is_empty() {
							let cur = g.ranks.iter().rposition(|&(_, t)| dp >= t);
							help_icon(ui, &p, |ui| {
								let rows: Vec<[String; 2]> = g.ranks.iter().map(|&(r, t)| [r.to_string(), t.to_string()]).collect();
								tip_table(ui, &p, "Detective ranks", ["Rank", "from DP"], &rows, cur);
							});
						}
					});
					if !g.ranks.is_empty() {
						let dp = save::u32_at(d, g.dp);
						let cur = g.ranks.iter().rposition(|&(_, t)| dp >= t);
						ui.horizontal(|ui| {
							label(ui, LW, "Detective Rank");
							ui.allocate_ui_with_layout(Vec2::new(FW, 26.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
								ui.set_min_width(FW);
								ui.label(egui::RichText::new(cur.map_or("—", |i| g.ranks[i].0)).font(f_semi(14.0)).color(p.text));
							});
							if let Some(next) = g.ranks.get(cur.map_or(0, |i| i + 1)) {
								ui.label(egui::RichText::new(format!("{1} more to {0}", next.0, next.1 - dp)).font(f_body(12.5)).color(p.muted));
							}
						});
					}
					head(ui, "Sepith");
					egui::Grid::new("sepith").num_columns(4).spacing([GAP, 6.0]).show(ui, |ui| {
						for (k, nm) in SEPITH.iter().enumerate() {
							ui.allocate_ui_with_layout(Vec2::new(LW, 26.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
								ui.set_min_width(LW);
								sepith_icon(ui, &atlas, k, 24.0);
								ui.label(egui::RichText::new(*nm).font(f_body(13.0)).color(p.muted));
							});
							let o = g.sepith + k * 4;
							let mut v = save::u32_at(d, o);
							if num(ui, &mut v, 99_999, 10.0) {
								save::set_u32(d, o, v);
							}
							if k % 2 == 1 {
								ui.end_row();
							}
						}
					});
				});
				if wide {
					ui.add_space(COL_GAP - GAP);
				}
				ui.vertical(|ui| {
					ui.set_width(RIGHT_W);
					head(ui, "Combat Notebook");
					egui::Grid::new("battle").num_columns(4).spacing([GAP, 6.0]).show(ui, |ui| {
						for (i, (nm, off)) in save::BATTLE_STATS.iter().enumerate() {
							label(ui, RW, nm);
							let at = g.battle + off;
							let mut v = save::u16_at(d, at);
							if num(ui, &mut v, 65_535, 1.0) {
								save::set_u16(d, at, v);
							}
							if i % 2 == 1 {
								ui.end_row();
							}
						}
					});
				});
			});
			ui.with_layout(row_layout, |ui| {
				ui.vertical(|ui| {
					ui.set_width(LEFT_W);
					head(ui, "Play time");
					let t = save::u32_at(d, g.time);
					let (mut h, mut m, mut s) = (t / 3600, t % 3600 / 60, t % 60);
					ui.horizontal(|ui| {
						label(ui, LW, "Playtime");
						let w3 = (FW * 2.0 + LW + GAP * 2.0 - GAP * 2.0) / 3.0;
						let mut ch = false;
						ch |= ui.add_sized([w3, 26.0], egui::DragValue::new(&mut h).clamp_existing_to_range(false).range(0..=9_999).suffix(" h")).on_hover_text(num_hint()).changed();
						ch |= ui.add_sized([w3, 26.0], egui::DragValue::new(&mut m).clamp_existing_to_range(false).range(0..=59).suffix(" min")).on_hover_text(num_hint()).changed();
						ch |= ui.add_sized([w3, 26.0], egui::DragValue::new(&mut s).clamp_existing_to_range(false).range(0..=59).suffix(" s")).on_hover_text(num_hint()).changed();
						if ch {
							save::set_u32(d, g.time, h * 3600 + m * 60 + s);
						}
					});
				});
				if wide {
					ui.add_space(COL_GAP - GAP);
				}
				ui.vertical(|ui| {
					ui.set_width(RIGHT_W);
					head(ui, "Difficulty");
					let mut v = save::difficulty(g, d);
					let v0 = v;
					ui.horizontal(|ui| {
						label(ui, RW, "Difficulty");
						let cur = save::DIFFICULTY.iter().find(|x| x.0 == v).map_or_else(|| format!("? ({v})"), |x| x.1.to_string());
						egui::ComboBox::from_id_salt("diff").width(FW - 8.0).selected_text(cur).show_ui(ui, |ui| {
							for (k, t) in save::DIFFICULTY {
								ui.selectable_value(&mut v, k, t);
							}
						});
					});
					if v != v0 {
						save::set_difficulty(g, d, v);
					}
				});
			});
			ui.add_space(10.0);
			ui.label(egui::RichText::new("The game updates the time and difficulty in the slot description on the next save.").font(f_body(12.0)).color(p.muted));
		});
	}

	fn lineup_card(&self, ui: &mut egui::Ui, tex: &Option<egui::TextureHandle>, name: &str, tag: &str, big: bool, selected: bool, target: bool) -> egui::Response {
		let p = self.pal;
		let size = card_size(big);
		let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
		let pt = ui.painter();
		pt.rect_filled(rect, 8, if resp.hovered() || target { p.div2 } else { p.bg });
		let stroke = if selected || target { Stroke::new(2.0, p.accent) } else { Stroke::new(1.0, p.div) };
		pt.rect_stroke(rect, 8, stroke, StrokeKind::Inside);
		let face = if big { 84.0 } else { 64.0 };
		let fr = Rect::from_center_size(Pos2::new(rect.center().x, rect.min.y + 8.0 + face / 2.0), Vec2::splat(face));
		if let Some(t) = tex {
			egui::Image::new(t).corner_radius(6).paint_at(ui, fr);
		}
		let ny = fr.max.y + 12.0;
		paint_text_clipped(ui, Rect::from_center_size(Pos2::new(rect.center().x, ny), Vec2::new(size.x - 10.0, 18.0)), name, f_body(13.0), p.text, Align2::CENTER_CENTER);
		let col = if tag.starts_with('⚠') { p.warn } else { p.muted };
		paint_text_clipped(ui, Rect::from_center_size(Pos2::new(rect.center().x, ny + 16.0), Vec2::new(size.x - 10.0, 16.0)), tag, f_body(11.0), col, Align2::CENTER_CENTER);
		if resp.drag_started() {
			egui::DragAndDrop::set_payload(ui.ctx(), name.to_string());
		}
		resp.on_hover_cursor(egui::CursorIcon::Grab)
	}

	fn lineup(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
		let g = self.g;
		let p = self.pal;
		let (list, have, names) = {
			let d = self.data.as_ref().unwrap();
			let list = save::party(g, d);
			let have: Vec<u32> = g.playable.iter().map(|&k| k as u32).filter(|c| !list.contains(c) && save::u16_at(d, save::chr_at(g, *c as usize) + field::LEVEL) > 0).collect();
			let names: HashMap<u32, u32> = list.iter().chain(have.iter()).map(|&c| (c, self.party_name_id(d, c))).collect();
			(list, have, names)
		};
		let was = save::party(g, &self.orig);
		let at = |pos: LPos| match pos {
			LPos::Line(i) => list.get(i).copied(),
			LPos::Avail(j) => have.get(j).copied(),
		};
		if self.lsel.is_some_and(|s| at(s).is_none()) {
			self.lsel = None;
		}
		let faces: HashMap<u32, Option<egui::TextureHandle>> = names.values().map(|&n| (n, self.face(ctx, n))).collect();
		let dragging = egui::DragAndDrop::has_any_payload(ctx);
		let mut act: Option<(LPos, Result<LPos, u8>)> = None;
		let mut zone_act: Option<(LPos, Result<LPos, u8>)> = None;
		let mut drag_from: Option<LPos> = None;
		let zone = |ui: &mut egui::Ui, title: &str, right: &str, add: &mut dyn FnMut(&mut egui::Ui)| -> egui::Response {
			let r = egui::Frame::new().fill(p.surface).corner_radius(10).inner_margin(egui::Margin::symmetric(14, 10)).show(ui, |ui| {
				ui.set_width(ui.available_width());
				ui.horizontal(|ui| {
					ui.label(egui::RichText::new(title).font(f_semi(13.5)).color(p.text));
					ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| ui.label(egui::RichText::new(right).font(f_body(12.0)).color(p.muted)));
				});
				ui.add_space(4.0);
				ui.horizontal_wrapped(|ui| {
					ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
					add(ui)
				});
			});
			ui.add_space(8.0);
			ui.interact(r.response.rect, ui.id().with(("zone", title)), Sense::hover())
		};
		let mut card_at = |this: &Self, ui: &mut egui::Ui, pos: LPos, c: u32, big: bool, tag: &str| {
			let nid = names.get(&c).copied().unwrap_or(c);
			let name = this.who_name(nid);
			let selected = this.lsel == Some(pos);
			let hovered_drop = dragging && ui.rect_contains_pointer(Rect::from_min_size(ui.cursor().min, card_size(big)));
			let r = this.lineup_card(ui, faces.get(&nid).unwrap_or(&None), &name, tag, big, selected, hovered_drop);
			if r.drag_started() || r.dragged() {
				drag_from = Some(pos);
			}
			if r.dnd_release_payload::<String>().is_some() {
				if let Some(from) = this.drag_src {
					act = Some((from, Ok(pos)));
				}
			}
			if r.clicked() {
				act = Some((pos, Err(255)));
			}
		};
		ui.label(egui::RichText::new("Drag a card, or click two cards to swap them. Drag a card into “Can be added” to remove it from the party.").font(f_body(12.5)).color(p.muted));
		ui.add_space(6.0);
		egui::ScrollArea::vertical().id_salt("lineup").auto_shrink([false, false]).show(ui, |ui| {
			let tag = |c: u32, zone: u8| {
				if c as usize >= save::CHARS {
					"guest"
				} else if zone < 2 && !was.contains(&c) {
					"⚠ wasn't in party"
				} else {
					["in battle", "reserve", "not in party"][zone as usize]
				}
			};
			let z0 = zone(ui, "In battle", "4 slots", &mut |ui| {
				for i in 0..save::PARTY_ACTIVE {
					match list.get(i) {
						Some(&c) => card_at(self, ui, LPos::Line(i), c, true, tag(c, 0)),
						None => {
							let (rect, _) = ui.allocate_exact_size(card_size(true), Sense::hover());
							ui.painter().rect_stroke(rect, 8, Stroke::new(1.0, p.div2), StrokeKind::Inside);
							ui.painter().text(rect.center(), Align2::CENTER_CENTER, "empty", f_body(12.0), p.muted);
						}
					}
				}
			});
			let reserve = list.len().saturating_sub(save::PARTY_ACTIVE);
			let z1 = zone(ui, "Reserve", &format!("{reserve} of 4"), &mut |ui| {
				for i in save::PARTY_ACTIVE..list.len() {
					card_at(self, ui, LPos::Line(i), list[i], false, tag(list[i], 1));
				}
				if reserve == 0 {
					ui.label(egui::RichText::new("drag here").font(f_body(12.0)).color(p.muted));
				}
			});
			let z2 = zone(ui, "Can be added", "characters who have joined", &mut |ui| {
				for (j, &c) in have.iter().enumerate() {
					card_at(self, ui, LPos::Avail(j), c, false, tag(c, 2));
				}
				if have.is_empty() {
					ui.label(egui::RichText::new("everyone is already in the party").font(f_body(12.0)).color(p.muted));
				}
			});
			for (k, z) in [z0, z1, z2].iter().enumerate() {
				if z.dnd_release_payload::<String>().is_some() {
					if let Some(from) = self.drag_src {
						zone_act = Some((from, Err(k as u8)));
					}
				}
			}
			ui.add_space(4.0);
			ui.label(egui::RichText::new("Guests (KeA, Renne…) are added and removed by the story.").font(f_body(12.0)).color(p.muted));
			ui.label(egui::RichText::new("⚠ The game loads models and animations per story chapter: a character who isn't in the party at this point may show up with someone else's model or none. Test on a new slot.").font(f_body(12.0)).color(p.warn));
		});
		if act.is_none() {
			act = zone_act;
		}
		if let Some(f) = drag_from {
			self.drag_src = Some(f);
		}
		if !dragging && drag_from.is_none() && act.is_none() {
			self.drag_src = None;
		}
		if let (Some(from), Some(pos)) = (self.drag_src, ctx.pointer_interact_pos()) {
			if dragging {
				if let Some(c) = at(from) {
					let layer = egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("lineup_drag"));
					let pt = ctx.layer_painter(layer);
					let r = Rect::from_min_size(pos + Vec2::new(12.0, 8.0), Vec2::new(150.0, 26.0));
					pt.rect(r, 6, p.surface, Stroke::new(1.0, p.accent), StrokeKind::Inside);
					pt.text(r.left_center() + Vec2::new(10.0, 0.0), Align2::LEFT_CENTER, self.who_name(names.get(&c).copied().unwrap_or(c)), f_body(13.0), p.text);
				}
			}
		}
		let Some((a, b)) = act else { return };
		let mut new = list.clone();
		match b {
			Err(255) => match self.lsel.take() {
				None => self.lsel = Some(a),
				Some(s) if s == a => {}
				Some(s) => lineup_swap(&mut new, &have, s, a),
			},
			Ok(t) => lineup_swap(&mut new, &have, a, t),
			Err(z) => lineup_move(&mut new, &have, a, z),
		}
		if b != Err(255) {
			self.lsel = None;
			self.drag_src = None;
		}
		if new != list {
			save::set_party(g, self.data.as_mut().unwrap(), &new);
		}
	}

	fn inventory(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
		let g = self.g;
		let p = self.pal;
		let atlas = self.icons(ctx);
		let tabs: Vec<&str> = save::TABS.iter().map(|t| t.0).collect();
		let mut r = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(ui.max_rect().min, Vec2::new(ui.max_rect().width(), 32.0))));
		segmented(&mut r, &p, "itab", &tabs, &mut self.item_tab, None, 32.0, 8, f_body(12.0), &[]);
		ui.add_space(42.0);
		ui.horizontal(|ui| {
			ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Search by name").desired_width(260.0));
			ui.add_space(12.0);
			checkbox(ui, &p, &mut self.owned_only, "Owned only", 16.0, f_body(13.0));
		});
		ui.add_space(6.0);
		let q = self.search.trim().to_lowercase();
		let tab = self.item_tab;
		let d = self.data.as_mut().unwrap();
		let shown = |it: &save::Item, d: &[u8]| save::tab_of(it) == Some(tab) && (q.is_empty() || it.name.to_lowercase().contains(&q)) && (!self.owned_only || save::item_count(g, d, it.id) > 0);
		let mut groups: Vec<(String, String, Option<u32>, Vec<&save::Item>)> = Vec::new();
		let mut list: Vec<&save::Item> = self.items.iter().filter(|it| shown(it, d)).collect();
		list.sort_by_key(|it| (if it.cell == 22 { 19 } else { it.cell }, it.element, it.id));
		for it in list {
			let (key, title, icon) = match it.cell {
				1..=10 => {
					let rec = (it.owner as usize).saturating_sub(1);
					let who = if it.owner == 0 { "Anyone".to_string() } else { short_name(&self.names, save::record_name(g, d, rec)) };
					(format!("{}:w{}", g.dir, it.owner), who, it.icon())
				}
				18 | 26 => {
					let e = it.element.clamp(1, 7) as usize;
					let title = if it.cell == 26 { "Master Quartz".to_string() } else { SEPITH[e - 1].to_string() };
					(format!("{}:q{}:{e}", g.dir, it.cell), title, it.icon())
				}
				c => {
					let title = match c {
						11 => "Armor",
						12 => "Footwear",
						13 => "Accessories",
						14 => "Recovery",
						19 | 22 => "Food",
						23 => "Bait",
						24 => "Fish",
						25 => "Rods",
						_ => "",
					};
					(format!("{}:k{title}", g.dir), title.to_string(), if title.is_empty() { None } else { Some(c) })
				}
			};
			match groups.iter_mut().find(|gr| gr.0 == key) {
				Some(gr) => gr.3.push(it),
				None => groups.push((key, title, icon, vec![it])),
			}
		}
		let mut set_all = None;
		ui.horizontal(|ui| {
			ui.label(egui::RichText::new("All visible:").font(f_body(13.0)).color(p.muted));
			for n in [0u32, 1, 99] {
				if small_button(ui, &p, &format!("All “{n}”"), None, 26.0, !groups.is_empty(), p.text).clicked() {
					set_all = Some(n);
				}
			}
		});
		if let Some(n) = set_all {
			for gr in &groups {
				for it in &gr.3 {
					save::set_item_count(g, d, it.id, n.min(it.max));
				}
			}
		}
		ui.add_space(6.0);
		let cols = ((ui.available_width() / 240.0).floor() as usize).max(1);
		let cell_w = ui.available_width() / cols as f32;
		let item = |ui: &mut egui::Ui, d: &mut Vec<u8>, it: &save::Item| {
			let (rect, _) = ui.allocate_exact_size(Vec2::new(cell_w - 8.0, 30.0), Sense::hover());
			let fill = if p.dark { p.surface } else { Color32::from_black_alpha(14) };
			ui.painter().rect_filled(rect, 6, fill);
			let field = Rect::from_min_max(Pos2::new(rect.max.x - 46.0, rect.min.y), rect.max).shrink(1.0);
			let mut n = save::item_count(g, d, it.id);
			let mut fc = ui.new_child(egui::UiBuilder::new().max_rect(field));
			{
				let v = &mut fc.style_mut().visuals.widgets;
				for w in [&mut v.inactive, &mut v.hovered, &mut v.active, &mut v.open] {
					w.bg_stroke = Stroke::NONE;
					w.corner_radius = egui::CornerRadius { nw: 0, sw: 0, ne: 5, se: 5 };
					w.expansion = 0.0;
				}
			}
			if fc.add_sized(field.size(), egui::DragValue::new(&mut n).clamp_existing_to_range(false).range(0..=it.max)).on_hover_text(num_hint()).changed() {
				save::set_item_count(g, d, it.id, n);
			}
			ui.painter().line_segment([Pos2::new(field.min.x, rect.min.y), Pos2::new(field.min.x, rect.max.y)], Stroke::new(1.0, p.div));
			ui.painter().rect_stroke(rect, 6, Stroke::new(1.0, p.div), StrokeKind::Inside);
			let inner = rect.shrink2(Vec2::new(6.0, 0.0));
			let icon_r = Rect::from_min_size(Pos2::new(inner.min.x, inner.center().y - 12.0), Vec2::splat(24.0));
			if let (Some(t), Some(cell)) = (&atlas, it.icon()) {
				egui::Image::new(t).uv(atlas_uv(t, cell)).paint_at(ui, icon_r);
			}
			let name_r = Rect::from_min_max(Pos2::new(icon_r.max.x + 8.0, inner.min.y), Pos2::new(field.min.x - 8.0, inner.max.y));
			let mut nc = ui.new_child(egui::UiBuilder::new().max_rect(name_r).layout(egui::Layout::left_to_right(egui::Align::Center)));
			let r = nc.add(egui::Label::new(egui::RichText::new(&it.name).font(f_body(13.5)).color(p.text)).truncate().sense(Sense::click()));
			click_tip(&nc, &r, ("item", it.id), |ui| item_tip(ui, &p, it));
		};
		let headers = groups.len() > 1;
		egui::ScrollArea::vertical().id_salt(("items", self.item_tab, g.dir)).auto_shrink([false, false]).show(ui, |ui| {
			for (key, title, gicon, its) in &groups {
				let open = !self.collapsed.contains(key);
				if headers {
					ui.add_space(4.0);
					ui.horizontal(|ui| {
						if gicon.is_some() {
							atlas_cell(ui, &atlas, *gicon, 22.0);
						}
						let txt = format!("{} {title}", if open { "▾" } else { "▸" });
						let r = ui.add(egui::Label::new(egui::RichText::new(txt).font(f_semi(14.0)).color(p.text)).sense(Sense::click())).on_hover_cursor(egui::CursorIcon::PointingHand);
						ui.label(egui::RichText::new(its.len().to_string()).font(f_body(12.0)).color(p.muted));
						if r.clicked() {
							if open {
								self.collapsed.insert(key.clone());
							} else {
								self.collapsed.remove(key);
							}
						}
					});
				}
				if open {
					let gap = ui.spacing().item_spacing;
					ui.spacing_mut().item_spacing = Vec2::new(8.0, 4.0);
					for row in its.chunks(cols) {
						ui.horizontal(|ui| {
							for it in row {
								item(ui, d, it);
							}
						});
					}
					ui.spacing_mut().item_spacing = gap;
				}
			}
		});
		if groups.is_empty() {
			ui.label(egui::RichText::new(if self.owned_only { "Nothing in this section (uncheck “Owned only” to add items)" } else { "Nothing found" }).font(f_body(13.0)).color(p.muted));
		}
	}
}

fn short_name(names: &HashMap<u32, String>, who: u32) -> String {
	match names.get(&who).map(|s| s.as_str()) {
		Some("Sergeant Major Seeker") => "Noel".into(),
		Some(n) => n.strip_prefix("Detective ").or_else(|| n.strip_prefix("Divine Wolf ")).unwrap_or(n).to_string(),
		None => format!("#{who}"),
	}
}

fn item_tip(ui: &mut egui::Ui, p: &Pal, it: &save::Item) {
	ui.set_max_width(360.0);
	ui.label(egui::RichText::new(&it.name).font(f_semi(13.5)).color(p.text));
	let desc = it.desc.replace("\\n", "\n");
	if !desc.trim().is_empty() {
		ui.label(egui::RichText::new(desc.trim()).font(f_body(12.5)).color(p.muted));
	}
	if matches!(it.cell, 1..=13 | 18 | 26) {
		let parts: Vec<String> = save::STATS.iter().zip(it.stats.iter().map(|&v| v as i32).chain(it.sub.iter().map(|&v| v as i32))).filter(|(_, v)| *v != 0).map(|(n, v)| format!("{n} {v:+}")).collect();
		let flat: String = desc.chars().filter(|c| !c.is_whitespace()).collect();
		if !parts.is_empty() && !parts.iter().all(|s| flat.contains(&s.replace(' ', ""))) {
			ui.label(egui::RichText::new(parts.join("  ")).font(f_semi(12.5)).color(p.text));
		}
	}
}

impl eframe::App for App {
	fn update(&mut self, ctx: &egui::Context, _f: &mut eframe::Frame) {
		let p = self.pal;
		let slot = self.sel.and_then(|i| self.slots.get(i)).filter(|_| self.data.is_some()).map_or(String::new(), |s| format!(" — {}", s.name));
		let t = match self.game {
			Some(i) => format!("{} — {}{slot}", title(), GAMES[i].1),
			None => title().into(),
		};
		if t != self.shown_title {
			ctx.send_viewport_cmd(egui::ViewportCommand::Title(t.clone()));
			self.shown_title = t;
		}
		self.global_input(ctx);
		egui::CentralPanel::default().frame(egui::Frame::new().fill(p.bg)).show(ctx, |ui| if self.game.is_some() { self.ui(ui, ctx) } else { self.picker(ui, ctx) });
		self.windows(ctx);
		match show_dialog(ctx, &p, "Save Editor", &mut self.dialog) {
			Some(Act::Overwrite) => self.overwrite(),
			Some(Act::Close) => {
				self.allow_close = true;
				ctx.send_viewport_cmd(egui::ViewportCommand::Close);
			}
			Some(Act::Open(i)) => self.open(i),
			None => {}
		}
	}
	fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
		self.save_settings();
	}
}

fn editor_icon() -> egui::IconData {
	match image::load_from_memory(include_bytes!("icons/crossbell_save.png")) {
		Ok(im) => {
			let im = im.to_rgba8();
			egui::IconData { width: im.width(), height: im.height(), rgba: im.into_raw() }
		}
		Err(_) => window_icon(),
	}
}

fn main() {
	let opts = eframe::NativeOptions {
		viewport: egui::ViewportBuilder::default().with_title(TITLE).with_inner_size([1360.0, 780.0]).with_min_inner_size([960.0, 560.0]).with_icon(editor_icon()),
		centered: true,
		..Default::default()
	};
	let _ = eframe::run_native(TITLE, opts, Box::new(|cc| Ok(Box::new(App::new(cc)))));
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn lineup_moves() {
		let have = [4, 8];
		let mut l = vec![0, 1, 2, 3, 6];
		lineup_swap(&mut l, &have, LPos::Line(1), LPos::Line(4));
		assert_eq!(l, [0, 6, 2, 3, 1]);
		lineup_swap(&mut l, &have, LPos::Avail(0), LPos::Line(3));
		assert_eq!(l, [0, 6, 2, 4, 1]);
		lineup_move(&mut l, &have, LPos::Line(2), 2);
		assert_eq!(l, [0, 6, 4, 1]);
		lineup_move(&mut l, &have, LPos::Avail(1), 0);
		assert_eq!(l, [0, 6, 4, 1], "no free battle slots");
		lineup_move(&mut l, &have, LPos::Avail(1), 1);
		assert_eq!(l, [0, 6, 4, 1, 8]);
		lineup_move(&mut l, &have, LPos::Line(1), 1);
		assert_eq!(l, [0, 4, 1, 8, 6]);
	}
}
