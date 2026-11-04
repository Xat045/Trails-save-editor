#![cfg_attr(windows, windows_subsystem = "windows")]

mod gui_common;

use eframe::egui::{self, Align2, Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use gui_common::*;
use kiseki_tool::kai::save::{self, db1};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const TITLE: &str = concat!("Calvard Save Editor ", env!("VER_CALVARDSAVEEDITOR"), env!("VER_SUFFIX"));

fn title() -> &'static str {
	TITLE
}

const GAMES: [(&str, &str, bool); 3] = [("db1", "Trails through Daybreak", true), ("db2", "Trails through Daybreak II", true), ("horizon", "Trails beyond the Horizon", true)];

fn profile(game: usize) -> &'static save::Profile {
	match game {
		1 => &save::DB2,
		2 => &save::HZ,
		_ => &save::DB1,
	}
}
fn falcom_dir(game: usize) -> &'static str {
	match game {
		1 => "Kuro2",
		2 => "Kai",
		_ => "Kuro",
	}
}

const SEPITH: [&str; 8] = ["Earth", "Water", "Fire", "Wind", "Time", "Space", "Mirage", "Sepith Mass"];

#[derive(Clone)]
struct UiNode {
	p: String,
	x: f32,
	y: f32,
	w: f32,
	h: f32,
	pv: u8,
	a: f32,
	c: [f32; 3],
	uv: [f32; 4],
	t: String,
	f: Option<String>,
	fs: Option<f32>,
	ns: f32,
	sc: f32,
}

fn ui_parse(json: &[u8]) -> Vec<UiNode> {
	let v: serde_json::Value = serde_json::from_slice(json).unwrap_or_default();
	let num = |o: &serde_json::Value, k: &str| o.get(k).and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
	v.as_array()
		.map(|a| {
			a.iter()
				.map(|o| {
					let arr = |k: &str, n: usize| -> Vec<f32> { o.get(k).and_then(|x| x.as_array()).map(|a| a.iter().map(|x| x.as_f64().unwrap_or(1.0) as f32).collect()).unwrap_or_else(|| vec![if k == "c" { 1.0 } else { 0.0 }; n]) };
					let c = arr("c", 3);
					let uv = arr("uv", 4);
					UiNode {
						p: o.get("p").and_then(|x| x.as_str()).unwrap_or("").to_string(),
						x: num(o, "x"),
						y: num(o, "y"),
						w: num(o, "w"),
						h: num(o, "h"),
						pv: num(o, "pv") as u8,
						a: o.get("a").and_then(|x| x.as_f64()).unwrap_or(1.0) as f32,
						c: [c[0], c[1], c[2]],
						uv: [uv[0], uv[1], uv[2], uv[3]],
						t: o.get("t").and_then(|x| x.as_str()).unwrap_or("").to_string(),
						f: o.get("f").and_then(|x| x.as_str()).map(str::to_string),
						fs: o.get("fs").and_then(|x| x.as_f64()).map(|x| x as f32),
						ns: num(o, "ns"),
						sc: o.get("sc").and_then(|x| x.as_f64()).unwrap_or(1.0) as f32,
					}
				})
				.collect()
		})
		.unwrap_or_default()
}

fn pivot_frac(pv: u8) -> (f32, f32) {
	let ax = match pv {
		1 | 3 | 5 => 1.0,
		6..=8 => 0.5,
		_ => 0.0,
	};
	let ay = match pv {
		2 | 3 | 7 => 1.0,
		4 | 5 | 8 => 0.5,
		_ => 0.0,
	};
	(ax, ay)
}

#[allow(clippy::too_many_arguments)]
fn draw_digits(p: &egui::Painter, tex: &egui::TextureHandle, uv: [f32; 4], ns: f32, text: &str, at: Pos2, pv: u8, s: f32, tint: Color32) {
	let ts = tex.size_vec2();
	let (gw, gh) = (uv[2] / 10.0, uv[3]);
	let n = text.chars().count() as f32;
	let (ax, ay) = pivot_frac(pv);
	let mut x = at.x - (n * gw + (n - 1.0).max(0.0) * ns) * s * ax;
	let y = at.y - gh * s * ay;
	for ch in text.chars() {
		if let Some(dg) = ch.to_digit(10) {
			let u0 = Pos2::new((uv[0] + dg as f32 * gw) / ts.x, uv[1] / ts.y);
			let u1 = Pos2::new((uv[0] + (dg + 1) as f32 * gw) / ts.x, (uv[1] + gh) / ts.y);
			p.image(tex.id(), Rect::from_min_size(Pos2::new(x, y), Vec2::new(gw, gh) * s), Rect::from_min_max(u0, u1), tint);
		}
		x += (gw + ns) * s;
	}
}

#[allow(clippy::too_many_arguments)]
fn game_edit(ui: &mut egui::Ui, p: &egui::Painter, tex: &Option<egui::TextureHandle>, key: &str, uv: [f32; 4], ns: f32, at: Pos2, pv: u8, s: f32, v: &mut u32, max: u32) -> bool {
	let text = v.to_string();
	let (gw, gh) = (uv[2] / 10.0, uv[3]);
	let (ax, ay) = pivot_frac(pv);
	let w = (text.len().max(3) as f32 * gw + 8.0) * s;
	let r = Rect::from_min_size(Pos2::new(at.x - w * ax, at.y - gh * s * ay), Vec2::new(w, gh * s));
	let id = ui.id().with(("game_edit", key));
	let active = ui.ctx().data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
	let mut c = ui.new_child(egui::UiBuilder::new().max_rect(r).id_salt(id).layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight)));
	let vis = c.visuals_mut();
	vis.override_text_color = Some(if active { Color32::WHITE } else { Color32::TRANSPARENT });
	vis.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
	vis.widgets.inactive.bg_stroke = Stroke::NONE;
	vis.widgets.hovered.weak_bg_fill = Color32::from_black_alpha(200);
	vis.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_white_alpha(90));
	vis.widgets.active.weak_bg_fill = Color32::from_black_alpha(220);
	c.style_mut().override_font_id = Some(f_semi((gh * s * 0.75).max(11.0)));
	let speed = if max > 100_000 { 100.0 } else { 1.0 };
	let resp = c.add(egui::DragValue::new(v).clamp_existing_to_range(false).range(0..=max).speed(speed)).on_hover_text("Drag, or click and type a number");
	let now = resp.hovered() || resp.has_focus() || resp.dragged();
	ui.ctx().data_mut(|d| d.insert_temp(id, now));
	if !active {
		if let Some(t) = tex {
			draw_digits(p, t, uv, ns, &text, at, pv, s, Color32::WHITE);
		}
	}
	resp.changed()
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

fn tint_of(n: &UiNode) -> Color32 {
	let k = |v: f32| (v.clamp(0.0, 1.0) * 255.0) as u8;
	Color32::from_rgba_unmultiplied(k(n.c[0]), k(n.c[1]), k(n.c[2]), k(n.a))
}

const TEX_SMOOTH: egui::TextureOptions = egui::TextureOptions { magnification: egui::TextureFilter::Linear, minification: egui::TextureFilter::Linear, wrap_mode: egui::TextureWrapMode::ClampToEdge, mipmap_mode: None };

const GAME_TEXT: Color32 = Color32::from_rgb(236, 236, 236);
const GAME_MUTED: Color32 = Color32::from_rgb(180, 178, 169);
const GAME_GOLD: Color32 = Color32::from_rgb(250, 199, 117);
const GAME_CYAN: Color32 = Color32::from_rgb(110, 225, 235);

#[derive(Clone, Copy, PartialEq, Debug)]
enum LPos {
	Line(usize),
	Avail(usize),
}

fn lgc_chart(ui: &mut egui::Ui, p: &Pal, lv: [usize; 3], max: usize) {
	let (rect, _) = ui.allocate_exact_size(Vec2::new(300.0, 230.0), Sense::hover());
	let pt = ui.painter();
	let c = Pos2::new(rect.center().x - 10.0, rect.center().y + 6.0);
	let r = 86.0;
	let ang = [std::f32::consts::PI, -std::f32::consts::FRAC_PI_3, std::f32::consts::FRAC_PI_3];
	let at = |k: usize, f: f32| c + Vec2::angled(ang[k]) * r * f;
	let law = Color32::from_rgb(40, 130, 180);
	let chaos = Color32::from_rgb(190, 60, 140);
	pt.circle_stroke(c, r + 6.0, Stroke::new(2.0, p.text));
	for lvl in 1..=max {
		let f = lvl as f32 / max as f32;
		let pts = [at(0, f), at(1, f), at(2, f), at(0, f)];
		for w in pts.windows(2) {
			pt.extend(egui::Shape::dashed_line(&[w[0], w[1]], Stroke::new(1.0, p.div2), 4.0, 3.0));
		}
	}
	for k in 0..3 {
		pt.line_segment([c, at(k, 1.0)], Stroke::new(1.0, p.div2));
	}
	let f = |k: usize| (lv[k] as f32 / max as f32).max(0.06);
	let tri = vec![at(0, f(0)), at(1, f(1)), at(2, f(2))];
	pt.add(egui::Shape::convex_polygon(tri.clone(), Color32::from_rgba_unmultiplied(90, 90, 170, 150), Stroke::new(1.5, Color32::from_rgb(70, 70, 140))));
	for k in 0..3 {
		let q = at(k, 1.0);
		pt.add(egui::Shape::convex_polygon(vec![q + Vec2::new(0.0, -5.0), q + Vec2::new(5.0, 0.0), q + Vec2::new(0.0, 5.0), q + Vec2::new(-5.0, 0.0)], p.text, Stroke::NONE));
	}
	let label = |k: usize| if lv[k] >= max { format!("{max} MAX") } else { lv[k].to_string() };
	pt.text(at(0, 1.0) + Vec2::new(-14.0, -10.0), Align2::RIGHT_BOTTOM, "LAW", f_semi(13.0), law);
	pt.text(at(0, 1.0) + Vec2::new(-14.0, 6.0), Align2::RIGHT_TOP, label(0), f_semi(15.0), law);
	pt.text(at(1, 1.0) + Vec2::new(10.0, -8.0), Align2::LEFT_BOTTOM, format!("GRAY {}", label(1)), f_semi(13.0), p.text);
	pt.text(at(2, 1.0) + Vec2::new(10.0, 8.0), Align2::LEFT_TOP, format!("CHAOS {}", label(2)), f_semi(13.0), chaos);
}

#[allow(clippy::too_many_arguments)]
fn lgc_game(ui: &mut egui::Ui, bg: &egui::TextureHandle, ch: &egui::TextureHandle, lv: [usize; 3], max: usize, digits: ([f32; 4], &str), k: f32) {
	let (rect, _) = ui.allocate_exact_size(Vec2::new(576.0, 320.0) * k, Sense::hover());
	let pt = ui.painter_at(rect);
	let ts = ch.size_vec2();
	let uv = |x: f32, y: f32, w: f32, h: f32| Rect::from_min_size(Pos2::new(x / ts.x, y / ts.y), Vec2::new(w / ts.x, h / ts.y));
	pt.image(bg.id(), rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
	let g = 0.7;
	let c = rect.min + Vec2::new(414.0, 134.0) * k;
	let at = |x: f32, y: f32| c + Vec2::new(x, y) * g * k;
	let sprite = |cx: f32, cy: f32, u: Rect, w: f32, h: f32, tint: Color32| pt.image(ch.id(), Rect::from_center_size(at(cx, cy), Vec2::new(w, h) * g * k), u, tint);
	sprite(-1.67, -3.33, uv(0.0, 318.0, 321.0, 319.0), 321.0, 319.0, Color32::WHITE);
	let o = Vec2::new(0.5, -0.5);
	let tip = [Vec2::new(-136.5, -6.5), Vec2::new(72.5, -113.5), Vec2::new(64.5, 118.5)];
	let tri: Vec<Pos2> = (0..3).map(|i| at(o.x, o.y) + (tip[i] - o) * (lv[i].min(max) as f32 / max as f32).max(0.04) * g * k).collect();
	pt.add(egui::Shape::convex_polygon(tri, Color32::from_rgba_unmultiplied(84, 86, 140, 235), Stroke::NONE));
	sprite(0.0, 0.0, uv(0.0, 0.0, 321.0, 319.0), 321.0, 319.0, Color32::WHITE);
	type Grp = ((f32, f32), f32, [f32; 3], (f32, f32), (f32, f32));
	let groups: [Grp; 3] = [
		((-227.0, -56.0), 9.0, [0.13, 0.4, 0.56], (27.5, 24.0), (12.0, 80.0)),
		((39.0, -165.0), 44.0, [0.33, 0.33, 0.33], (72.5, 9.0), (102.0, 40.0)),
		((8.0, 140.0), 79.0, [0.56, 0.26, 0.49], (89.5, -22.0), (119.0, 8.0)),
	];
	for (i, &((gx, gy), ty, col, (nx, ny), (mx, my))) in groups.iter().enumerate() {
		let tint = Color32::from_rgb((col[0] * 255.0) as u8, (col[1] * 255.0) as u8, (col[2] * 255.0) as u8);
		sprite(gx + 43.0, gy + 17.0, uv(426.0, ty, 86.0, 34.0), 86.0, 34.0, Color32::WHITE);
		let d = lv[i].min(max);
		let ([dx, dy, dw, dh], order) = digits;
		let gw = dw / order.chars().count() as f32;
		if let Some(slot) = order.chars().position(|c| c.to_digit(10) == Some(d as u32)) {
			pt.image(ch.id(), Rect::from_min_size(at(gx + nx, gy + ny), Vec2::new(gw, dh) * g * k), uv(dx + slot as f32 * gw, dy, gw, dh), tint);
		}
		if d >= max {
			pt.image(ch.id(), Rect::from_min_size(at(gx + mx, gy + my), Vec2::new(46.0, 24.0) * g * k), uv(513.0, 9.0, 46.0, 24.0), tint);
		}
	}
}

fn atlas_uv(t: &egui::TextureHandle, c: u32) -> Rect {
	let s = t.size();
	let (w, h) = (48.0 / s[0] as f32, 48.0 / s[1] as f32);
	Rect::from_min_size(Pos2::new((c % 21) as f32 * w, (c / 21) as f32 * h), Vec2::new(w, h))
}

fn gtext(ui: &egui::Ui, p: &egui::Painter, pos: Pos2, align: Align2, t: &str, font: egui::FontId, col: Color32) {
	let job = egui::text::LayoutJob::single_section(t.to_string(), egui::TextFormat { font_id: font, color: col, italics: true, ..Default::default() });
	let g = ui.fonts(|f| f.layout_job(job));
	let r = align.anchor_size(pos, g.size());
	p.galley(r.min, g, col);
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

const VAN_STAYS: &str = "Van is always in the party — he can't be removed";

fn lineup_swap(list: &mut Vec<u32>, have: &[u32], a: LPos, b: LPos) -> Option<&'static str> {
	match (a, b) {
		(LPos::Line(i), LPos::Line(j)) => list.swap(i, j),
		(LPos::Line(i), LPos::Avail(j)) | (LPos::Avail(j), LPos::Line(i)) => {
			if list[i] == 0 {
				return Some(VAN_STAYS);
			}
			list[i] = have[j];
		}
		(LPos::Avail(_), LPos::Avail(_)) => {}
	}
	None
}

fn lineup_move(list: &mut Vec<u32>, have: &[u32], a: LPos, zone: u8) -> Option<&'static str> {
	let c = match a {
		LPos::Line(i) => list[i],
		LPos::Avail(j) => have[j],
	};
	match (a, zone) {
		(LPos::Line(_), 2) if c == 0 => return Some(VAN_STAYS),
		(LPos::Line(i), 2) => {
			list.remove(i);
		}
		(LPos::Line(i), 1) => {
			let c = list.remove(i);
			list.push(c);
		}
		(LPos::Avail(_), 0) if list.len() >= db1::LINEUP_ACTIVE => {}
		(LPos::Avail(_), 0 | 1) if list.len() < db1::LINEUP_MAX => list.push(c),
		_ => {}
	}
	None
}

struct Slot {
	name: String,
	dir: PathBuf,
	title: String,
	sub: String,
	when: String,
	playtime: String,
	chapter: String,
}

fn read_slots(root: &Path) -> Vec<Slot> {
	let mut v: Vec<Slot> = std::fs::read_dir(root)
		.into_iter()
		.flatten()
		.flatten()
		.filter(|e| e.path().join("user.dat").is_file())
		.filter_map(|e| {
			let name = e.file_name().to_string_lossy().to_string();
			if !name.starts_with("save") {
				return None;
			}
			let j: serde_json::Value = std::fs::read_to_string(e.path().join("detail.json")).ok().and_then(|s| serde_json::from_str(s.trim_start_matches('\u{feff}')).ok()).unwrap_or_default();
			let s = |k: &str| j.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
			let t = &j["Time"];
			let n = |k: &str| t.get(k).and_then(|x| x.as_u64()).unwrap_or(0);
			let detail = s("detail");
			let playtime = detail
				.lines()
				.filter_map(|l| l.split_once(": "))
				.find(|(_, v)| {
					let v = v.trim();
					v.len() >= 7 && v.chars().all(|c| c.is_ascii_digit() || c == ':') && v.matches(':').count() == 2
				})
				.map(|(_, v)| {
					let v = v.trim();
					let (h, rest) = v.split_once(':').unwrap_or((v, ""));
					format!("{}:{rest}", h.parse::<u32>().unwrap_or(0))
				})
				.unwrap_or_default();
			let chapter = detail.lines().next().unwrap_or("").trim().to_string();
			Some(Slot {
				name,
				dir: e.path(),
				title: s("title"),
				sub: s("subtitle"),
				when: if n("year") > 0 { format!("{:02}.{:02}.{} {:02}:{:02}", n("day"), n("month"), n("year"), n("hour"), n("minute")) } else { String::new() },
				playtime,
				chapter,
			})
		})
		.collect();
	v.sort_by(|a, b| a.name.cmp(&b.name));
	v
}

fn default_dir(game: usize) -> String {
	std::env::var_os("USERPROFILE").map(|h| PathBuf::from(h).join("Saved Games").join("Falcom").join(falcom_dir(game))).filter(|p| p.is_dir()).map(|p| p.display().to_string()).unwrap_or_default()
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

#[derive(Clone)]
enum Act {
	Overwrite,
	Close,
	Open(usize),
}

fn dir_signature(root: &Path) -> u64 {
	use std::hash::{Hash, Hasher};
	let mut h = std::collections::hash_map::DefaultHasher::new();
	let mut v: Vec<(String, u64)> = std::fs::read_dir(root)
		.into_iter()
		.flatten()
		.flatten()
		.filter_map(|e| {
			let m = std::fs::metadata(e.path().join("user.dat")).ok()?;
			let t = m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
			Some((e.file_name().to_string_lossy().to_string(), t))
		})
		.collect();
	v.sort();
	v.hash(&mut h);
	h.finish()
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
	items: Vec<save::Item>,
	names: HashMap<u32, (String, String)>,
	skills: HashMap<u32, save::Skill>,
	kinds: HashMap<u8, String>,
	dlcs: Vec<save::Dlc>,
	qlines: HashMap<(u32, usize), ([u8; 4], usize)>,
	collapsed: std::collections::HashSet<String>,
	models: HashMap<u32, u32>,
	art_tex: HashMap<String, Option<egui::TextureHandle>>,
	ui_lay: HashMap<String, std::rc::Rc<Vec<UiNode>>>,
	game: Option<usize>,
	g: &'static save::Profile,
	owners: HashMap<usize, u32>,
	faces: HashMap<u32, Vec<u8>>,
	face_tex: HashMap<u32, Option<egui::TextureHandle>>,
	icons: Option<Option<egui::TextureHandle>>,
	status: String,
	dialog: Dialog<Act>,
	zoom: f32,
	about: bool,
	tips: bool,
	show_service: bool,
	allow_close: bool,
	scan_at: f64,
	scan_sig: u64,
	scan_dir: String,
	gsearch: String,
	gsearch_open: bool,
	scroll_sel: bool,
	shown_title: String,
}

impl App {
	fn new(cc: &eframe::CreationContext) -> App {
		setup_fonts(&cc.egui_ctx);
		egui_extras::install_image_loaders(&cc.egui_ctx);
		let st = Settings::load("Calvard-SaveEditor.ini", "Calvard-SaveEditor");
		let pal = Pal::get(st.get("dark") != Some("0"));
		apply_visuals(&cc.egui_ctx, &pal);
		let dir = st.get("dir").map(|s| s.to_string()).filter(|s| !s.is_empty()).unwrap_or_else(|| default_dir(0));
		let mut app = App {
			pal,
			dir,
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
			owners: HashMap::new(),
			faces: HashMap::new(),
			face_tex: HashMap::new(),
			icons: None,
			items: Vec::new(),
			names: HashMap::new(),
			skills: HashMap::new(),
			models: HashMap::new(),
			kinds: HashMap::new(),
			dlcs: Vec::new(),
			qlines: HashMap::new(),
			collapsed: std::collections::HashSet::new(),
			art_tex: HashMap::new(),
			ui_lay: HashMap::new(),
			game: None,
			g: &save::DB1,
			status: "Select a slot on the left. Before writing, the program backs the slot up to the «Backups» folder next to it.".into(),
			dialog: Dialog::None,
			zoom: 1.0,
			about: false,
			tips: false,
			show_service: false,
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
		app.show_service = app.st.get("show_service") == Some("1");
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
		match self.game {
			Some(1) => format!("{k}2"),
			Some(2) => format!("{k}3"),
			_ => k.to_string(),
		}
	}

	fn set_game(&mut self, i: usize) {
		self.save_settings();
		self.game = Some(i);
		self.st.set("last_game", &i.to_string());
		let g = profile(i);
		self.g = g;
		self.items = save::items(save::table(g.items_tsv));
		self.names = save::names(save::table(g.names_tsv));
		for n in self.names.values_mut() {
			let short = n.0.strip_suffix(" the Immovable").or_else(|| n.0.strip_prefix("Security Chief ")).or_else(|| n.0.strip_prefix("Dominion ")).map(str::to_string);
			if let Some(s) = short {
				n.0 = s;
			}
		}
		self.skills = save::skills(save::table(g.skills_tsv));
		self.models = save::models(save::table(g.names_tsv));
		self.kinds = save::kinds(save::table(g.kinds_tsv));
		self.dlcs = save::dlcs(save::table(g.dlc_tsv));
		self.qlines = save::qlines(save::table(g.qline_tsv));
		self.owners = self.items.iter().map(|it| (it.id, it.owner)).collect();
		self.faces = kiseki_tool::pack::get(g.faces).map(save::faces).unwrap_or_default();
		self.face_tex.clear();
		self.icons = None;
		self.art_tex.retain(|k, _| k.starts_with("cover:"));
		self.ui_lay.clear();
		self.data = None;
		self.sel = None;
		self.chr_sel = 0;
		self.lsel = None;
		self.drag_src = None;
		self.dir = self.st.get(&self.key("dir")).map(|s| s.to_string()).filter(|s| !s.is_empty()).unwrap_or_else(|| default_dir(i));
		self.item_tab = self.st.get(&self.key("item_tab")).and_then(|v| v.parse().ok()).unwrap_or(0).min(g.tabs.len());
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
		match save::read_for(self.g, &self.slots[i].dir.join("user.dat")) {
			Ok(d) => {
				self.orig = d.clone();
				self.data = Some(d);
				self.status = format!("Opened {}: {} — {}", self.slots[i].name, self.slots[i].title, self.slots[i].sub);
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
			None => {
				match save::read_for(self.g, file) {
					Ok(d) => {
						self.slots.push(Slot { name: slot.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(), dir: slot.to_path_buf(), title: "(opened file)".into(), sub: file.display().to_string(), when: String::new(), playtime: String::new(), chapter: String::new() });
						self.sel = Some(self.slots.len() - 1);
						self.orig = d.clone();
						self.data = Some(d);
						self.status = format!("Opened {}", file.display());
					}
					Err(e) => {
						self.wrong_game(&e);
						self.status = format!("✖ {e}");
					}
				}
			}
		}
		self.save_settings();
	}

	fn face(&mut self, ctx: &egui::Context, chr: u32) -> Option<egui::TextureHandle> {
		if let Some(t) = self.face_tex.get(&chr) {
			return t.clone();
		}
		let t = self.faces.get(&chr).and_then(|png| image::load_from_memory(png).ok()).map(|img| {
			let img = img.to_rgba8();
			ctx.load_texture(format!("face{chr}"), egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw()), TEX_SMOOTH)
		});
		self.face_tex.insert(chr, t.clone());
		t
	}

	fn art(&mut self, ctx: &egui::Context, name: &str) -> Option<egui::TextureHandle> {
		if let Some(t) = self.art_tex.get(name) {
			return t.clone();
		}
		let path = if let Some(c) = name.strip_prefix("cover:") {
			format!("covers/{c}.png")
		} else if let Some(f) = name.strip_prefix("ui:") {
			format!("{}/ui/{f}", self.ui_dir(f))
		} else {
			format!("{}/art/{name}.png", self.g.dir)
		};
		let t = kiseki_tool::pack::get(&path).and_then(|png| image::load_from_memory(png).ok()).map(|img| {
			let img = img.to_rgba8();
			ctx.load_texture(name, egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw()), TEX_SMOOTH)
		});
		self.art_tex.insert(name.to_string(), t.clone());
		t
	}

	fn ui_dir(&self, f: &str) -> &'static str {
		if self.g.dir == "Horizon" && !f.starts_with("lgc_") { "Daybreak-2" } else { self.g.dir }
	}

	fn ui_nodes(&mut self, screen: &str) -> std::rc::Rc<Vec<UiNode>> {
		let dir = self.ui_dir(screen);
		self.ui_lay.entry(screen.to_string()).or_insert_with(|| std::rc::Rc::new(kiseki_tool::pack::get(&format!("{dir}/ui/{screen}.json")).map(ui_parse).unwrap_or_default())).clone()
	}

	fn icons(&mut self, ctx: &egui::Context) -> Option<egui::TextureHandle> {
		self.icons
			.get_or_insert_with(|| {
				kiseki_tool::pack::get(self.g.icons).and_then(|png| image::load_from_memory(png).ok()).map(|img| {
					let img = img.to_rgba8();
					ctx.load_texture("icons", egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw()), TEX_SMOOTH)
				})
			})
			.clone()
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
		let backup = exe_dir().join("Backups").join(format!("{}_{}", slot.name, stamp()));
		if let Err(e) = copy_dir(&slot.dir, &backup) {
			self.dialog = Dialog::Info(format!("Could not back up the slot — it was not changed.\n{e}"));
			return;
		}
		match save::write(&slot.dir.join("user.dat"), &mut d) {
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
		let Some(n) = (0..1000).map(|n| format!("save{n:03}")).find(|n| !root.join(n).exists()) else { return };
		let to = root.join(&n);
		let from = self.slots[i].dir.clone();
		let res = (|| -> Result<(), String> {
			std::fs::create_dir_all(&to).map_err(|e| e.to_string())?;
			for f in ["detail.json", "icon0.png"] {
				if from.join(f).is_file() {
					std::fs::copy(from.join(f), to.join(f)).map_err(|e| e.to_string())?;
				}
			}
			save::write(&to.join("user.dat"), &mut d)
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

	fn picker(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
		let p = self.pal;
		let full = ui.max_rect();
		let covers: Vec<Option<egui::TextureHandle>> = GAMES.iter().map(|g| self.art(ctx, &format!("cover:{}", g.0))).collect();
		const ACCENT: [Color32; 3] = [Color32::from_rgb(0x4A, 0x8F, 0xE7), Color32::from_rgb(0xE5, 0x54, 0x3A), Color32::from_rgb(0x5C, 0xC3, 0xE0)];
		const ARC: [&str; 3] = ["CALVARD ARC · I", "CALVARD ARC · II", "CALVARD ARC · III"];
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
		let cw = (full.width() - 64.0).min(1320.0);
		let x0 = full.center().x - cw / 2.0;
		let gap = 24.0;
		let cols = (((cw + gap) / (280.0 + gap)).floor() as usize).clamp(1, GAMES.len());
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
							let y0 = ((1.0 - h) * 0.6).clamp(0.0, 1.0 - h);
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
		let fx0 = x0;
		match self.pick_game {
			Some(i) => {
				ui.painter().text(Pos2::new(fx0, foot.center().y - 9.0), Align2::LEFT_CENTER, "Selected", f_body(11.5), p.muted);
				ui.painter().text(Pos2::new(fx0, foot.center().y + 9.0), Align2::LEFT_CENTER, GAMES[i].1, f_semi(15.0), p.text);
			}
			None => {
				ui.painter().text(Pos2::new(fx0, foot.center().y), Align2::LEFT_CENTER, "Back up your save files before editing. This is an unofficial fan tool.", f_body(13.0), p.muted);
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
		let mut hits: Vec<(String, String, usize, usize)> = Vec::new();
		for k in 0..g.char_count {
			if save::u32_at(d, save::chr_at(g, k)) == 0 {
				continue;
			}
			let Some(c) = save::record_chr(g, d, k, &self.owners).filter(|c| g.addable.iter().any(|a| a.0 == *c)) else { continue };
			let who = self.chr_name(c);
			if who.to_lowercase().contains(q) {
				hits.push(("Character".into(), who.clone(), 1, k));
			}
			for cid in save::crafts(g, d, k) {
				if let Some(s) = self.skills.get(&cid).filter(|s| s.en.to_lowercase().contains(q) || s.ru.to_lowercase().contains(q)) {
					hits.push((format!("Craft · {who}"), s.en.clone(), 1, k));
				}
			}
		}
		for it in self.items.iter().filter(|it| it.id <= g.item_max_id && (self.show_service || !it.service()) && (it.en.to_lowercase().contains(q) || it.ru.to_lowercase().contains(q))) {
			let Some(t) = save::item_tab(g, it) else { continue };
			let n = save::item_count(g, d, it.id);
			hits.push((format!("{} · {n}", g.tabs[t]), it.en.clone(), 0, it.id));
		}
		let mut go = None;
		let area = egui::Area::new(egui::Id::new("gsearch_pop")).order(egui::Order::Foreground).fixed_pos(at.left_bottom() + Vec2::new(0.0, 4.0)).show(ctx, |ui| {
			egui::Frame::popup(ui.style()).show(ui, |ui| {
				ui.set_width(at.width().max(320.0));
				if hits.is_empty() {
					ui.label(egui::RichText::new("Nothing found").font(f_body(13.0)).color(p.muted));
				}
				egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
					for (i, (kind, name, _, _)) in hits.iter().enumerate().take(60) {
						let r = ui.add(egui::Button::new(egui::RichText::new(format!("{name}   ")).font(f_body(13.0)).color(p.text)).shortcut_text(egui::RichText::new(kind).font(f_body(11.5)).color(p.muted)).frame(false).min_size(Vec2::new(ui.available_width(), 24.0)));
						if r.clicked() {
							go = Some(i);
						}
					}
				});
			});
		});
		let pos = ctx.input(|i| i.pointer.interact_pos().unwrap_or_default());
		if ctx.input(|i| i.key_pressed(egui::Key::Escape)) || (ctx.input(|i| i.pointer.any_click()) && !area.response.rect.contains(pos) && !at.contains(pos)) {
			self.gsearch_open = false;
		}
		if let Some(i) = go {
			let (_, name, to, id) = hits[i].clone();
			self.gsearch_open = false;
			if to == 0 {
				if let Some(t) = self.items.iter().find(|it| it.id == id).and_then(|it| save::item_tab(g, it)) {
					self.item_tab = t;
				}
				self.tab = 2;
				self.search = name;
				self.owned_only = false;
			} else {
				self.tab = 1;
				self.chr_sel = id;
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
				ui.label(egui::RichText::new("Save editor for Trails through Daybreak, Daybreak II and beyond the Horizon (PC).").font(f_body(13.0)).color(p.muted));
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
				ui.add_space(10.0);
				let mut svc = self.show_service;
				if checkbox(ui, &p, &mut svc, "Show internal items (◆)", 16.0, f_body(13.0))
					.on_hover_text("Game entries marked with ◆: spare book slots, debug «Main Verification Slot» weapons, guest gear per chapter. Hidden by default.")
					.changed()
				{
					self.show_service = svc;
					self.st.set("show_service", if svc { "1" } else { "0" });
				}
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
					"Save slots are on the left; Up / Down arrows — next slot. You can drop a user.dat file onto the window.",
					"«General» — mira, sepith, difficulty, tokens, Total SP, L.G.C., play time.",
					"«Party» — drag a card onto another one, or click two cards, to swap them; drag into «Can be added» to remove from the party.",
					"«Items» — the amount of any item; untick «Owned only» to add a new one.",
					"Before writing, the slot is copied to «Backups» next to the program (About opens the folder).",
					"Ctrl+S — save to this slot, Ctrl+F — search across all tabs, Ctrl + mouse wheel — zoom.",
					"Numbers: drag left/right, or click and type.",
					"Internal game items (◆) are hidden — you can show them in «About».",
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
			let game = save::read(&path).ok().and_then(|d| (0..GAMES.len()).find(|&i| profile(i).size == d.len()));
			match game {
				Some(gi) => {
					if self.game != Some(gi) {
						self.pick_game = Some(gi);
						self.set_game(gi);
						self.shown_title.clear();
					}
					self.open_file(&path);
				}
				None => self.dialog = Dialog::Info(format!("{} is not a Calvard save.", path.display())),
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
			self.scroll_sel = true;
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
			let hint = format!("C:\\Users\\…\\Saved Games\\Falcom\\{}", falcom_dir(self.game.unwrap_or(0)));
			let (ch, cl, _) = path_field(&mut r, &p, "dir", &mut self.dir, 36.0, &hint, "Browse…", false, true);
			let br = Rect::from_min_size(Pos2::new(x0 + w - 360.0, y - 4.0), Vec2::new(360.0, 22.0));
			let mut b = ui.new_child(egui::UiBuilder::new().max_rect(br).layout(egui::Layout::right_to_left(egui::Align::Center)));
			if small_button(&mut b, &p, "Open save…", Some("file"), 22.0, true, p.accent).on_hover_text("Pick user.dat from any saveNNN folder").clicked() {
				if let Some(file) = pick_files(&self.dir, "Open save", "Trails through Daybreak save", &["dat"], false).into_iter().next() {
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
					ui.painter().text(Pos2::new(r.min.x + 10.0, r.min.y + 13.0), Align2::LEFT_CENTER, &s.name, f_mono(12.0), p.muted);
					ui.painter().text(Pos2::new(r.max.x - 10.0, r.min.y + 13.0), Align2::RIGHT_CENTER, &s.when, f_body(11.5), p.muted);
					let w = r.width() - 20.0;
					paint_text_clipped(ui, Rect::from_min_size(Pos2::new(r.min.x + 10.0, r.min.y + 24.0), Vec2::new(w, 18.0)), &s.title, f_semi(13.5), p.text, Align2::LEFT_CENTER);
					paint_text_clipped(ui, Rect::from_min_size(Pos2::new(r.min.x + 10.0, r.min.y + 43.0), Vec2::new(w, 17.0)), &s.sub, f_body(12.5), p.text, Align2::LEFT_CENTER);
					let pt = if s.playtime.is_empty() { String::new() } else { format!("Play time {}", s.playtime) };
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
		let (chapter, place) = self.sel.and_then(|i| self.slots.get(i)).map(|s| (s.chapter.clone(), s.sub.clone())).unwrap_or_default();
		let lgc_tex = self.art(ctx, "ui:lgc_bg.png").zip(self.art(ctx, "ui:lgc_chart.png"));
		let g = self.g;
		let d = self.data.as_mut().unwrap();
		const LW: f32 = 118.0;
		const FW: f32 = 96.0;
		let head = |ui: &mut egui::Ui, t: &str| {
			ui.add_space(14.0);
			ui.label(egui::RichText::new(t).font(f_semi(15.5)).color(p.text));
			ui.add_space(4.0);
		};
		let name = |ui: &mut egui::Ui, t: &str| {
			ui.allocate_ui_with_layout(Vec2::new(LW, 26.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
				ui.set_min_width(LW);
				ui.label(egui::RichText::new(t).font(f_body(13.0)).color(p.muted));
			});
		};
		egui::ScrollArea::vertical().id_salt("general").auto_shrink([false, false]).show(ui, |ui| {
		ui.add_space(4.0);
		ui.add(egui::Label::new(egui::RichText::new(if chapter.is_empty() { "—" } else { chapter.as_str() }).font(f_semi(26.0)).color(p.text)).wrap());
		if !place.is_empty() {
			ui.label(egui::RichText::new(&place).font(f_semi(14.0)).color(p.muted));
		}
		ui.add_space(2.0);
		let wide = ui.available_width() >= 1140.0;
		ui.with_layout(if wide { egui::Layout::left_to_right(egui::Align::Min) } else { egui::Layout::top_down(egui::Align::Min) }, |ui| {
			ui.spacing_mut().item_spacing.y = 10.0;
			ui.vertical(|ui| {
				ui.horizontal_top(|ui| {
					ui.vertical(|ui| {
						head(ui, "Mira");
						ui.horizontal(|ui| {
							name(ui, "Mira");
							let mut mira = save::u32_at(d, g.mira);
							if ui.add_sized([FW, 26.0], egui::DragValue::new(&mut mira).clamp_existing_to_range(false).range(0..=9_999_999).speed(100.0)).changed() {
								save::set_u32(d, g.mira, mira);
							}
						});
						if let Some(o) = g.s_token {
							ui.horizontal(|ui| {
								name(ui, g.token_name);
								let mut v = save::u32_at(d, o);
								if ui.add_sized([FW, 26.0], egui::DragValue::new(&mut v).clamp_existing_to_range(false).range(0..=9_999_999).speed(100.0)).changed() {
									save::set_u32(d, o, v);
								}
							});
						}
						if let Some(o) = g.casino {
							ui.horizontal(|ui| {
								name(ui, "Tokens");
								let mut v = save::u32_at(d, o);
								if ui.add_sized([FW, 26.0], egui::DragValue::new(&mut v).clamp_existing_to_range(false).range(0..=9_999_999).speed(10.0)).changed() {
									save::set_u32(d, o, v);
								}
							});
						}
					});
					ui.add_space(24.0);
					ui.vertical(|ui| {
						head(ui, "Difficulty");
						let (mut cmd, mut fld) = save::difficulty(g, d);
						let (c0, f0) = (cmd, fld);
						const CMD: [&str; 5] = ["Very Easy", "Easy", "Normal", "Hard", "Nightmare"];
						const FLD: [&str; 3] = ["Easy", "Normal", "Hard"];
						ui.horizontal(|ui| {
							name(ui, "Command Battle");
							egui::ComboBox::from_id_salt("diff_cmd").width(FW + 40.0).selected_text(CMD.get(cmd as usize).copied().unwrap_or("?")).show_ui(ui, |ui| {
								for (i, t) in CMD.iter().enumerate() {
									ui.selectable_value(&mut cmd, i as u32, *t);
								}
							});
						});
						ui.horizontal(|ui| {
							name(ui, "Field Battle");
							egui::ComboBox::from_id_salt("diff_fld").width(FW + 40.0).selected_text(FLD.get((fld as usize).wrapping_sub(1)).copied().unwrap_or("?")).show_ui(ui, |ui| {
								for (i, t) in FLD.iter().enumerate() {
									ui.selectable_value(&mut fld, i as u32 + 1, *t);
								}
							});
						});
						if (cmd, fld) != (c0, f0) {
							save::set_difficulty(g, d, cmd, fld);
						}
					});
				});
				head(ui, "Arkride Solutions");
				ui.horizontal(|ui| {
					name(ui, "Total SP");
					let mut sp = save::total_sp(g, d);
					let can = save::any_quest_done(g, d);
					let r = ui.add_enabled(can, egui::DragValue::new(&mut sp).clamp_existing_to_range(false).range(0..=9_999).speed(1.0));
					if r.changed() {
						save::set_total_sp(g, d, sp);
					}
					r.on_hover_text(if g.sp_extra.is_some() { "The game counts SP from completed quests plus an extra amount; the difference goes into that extra amount" } else { "The game counts SP from completed quests; the difference goes into those quests' bonuses" });
					let sp = save::total_sp(g, d);
					help_icon(ui, &p, |ui| {
						let cur = g.ranks.iter().rposition(|&(_, t)| sp >= t);
						let rows: Vec<[String; 2]> = g.ranks.iter().map(|&(r, t)| [r.to_string(), t.to_string()]).collect();
						tip_table(ui, &p, "Arkride Solutions ranks", ["Rank", "from SP"], &rows, cur);
					});
					ui.add_space(18.0);
					ui.label(egui::RichText::new("Office Rank").font(f_body(13.0)).color(p.muted));
					ui.add_space(6.0);
					ui.label(egui::RichText::new(save::office_rank(g, save::total_sp(g, d))).font(f_semi(15.0)).color(p.text));
				});
				head(ui, "Sepith");
				egui::Grid::new("sepith").num_columns(4).spacing([10.0, 6.0]).show(ui, |ui| {
					for (k, nm) in SEPITH.iter().enumerate() {
						ui.horizontal(|ui| {
							ui.set_width(LW);
							atlas_cell(ui, &atlas, Some(save::cells::SEPITH + k as u32), 24.0);
							ui.label(egui::RichText::new(*nm).font(f_body(13.0)).color(p.muted));
						});
						let o = g.sepith + k * 4;
						let mut v = save::u32_at(d, o);
						if ui.add_sized([FW, 26.0], egui::DragValue::new(&mut v).clamp_existing_to_range(false).range(0..=99_999).speed(10.0)).changed() {
							save::set_u32(d, o, v);
						}
						if k % 2 == 1 {
							ui.end_row();
						}
					}
				});
				head(ui, "Play time");
				let t = save::playtime(g, d);
				let (mut h, mut m, mut s) = ((t / 3600.0) as u32, (t % 3600.0 / 60.0) as u32, (t % 60.0) as u32);
				ui.horizontal(|ui| {
					name(ui, "Playtime");
					let mut ch = false;
					ch |= ui.add_sized([64.0, 26.0], egui::DragValue::new(&mut h).clamp_existing_to_range(false).range(0..=9_999).suffix(" h")).changed();
					ch |= ui.add_sized([64.0, 26.0], egui::DragValue::new(&mut m).clamp_existing_to_range(false).range(0..=59).suffix(" min")).changed();
					ch |= ui.add_sized([64.0, 26.0], egui::DragValue::new(&mut s).clamp_existing_to_range(false).range(0..=59).suffix(" s")).changed();
					if ch {
						save::set_playtime(g, d, (h * 3600 + m * 60 + s) as f64 + t.fract());
					}
				});
				ui.label(egui::RichText::new("The game updates the time in the slot description on the next save.").font(f_body(12.0)).color(p.muted));
			});
			ui.add_space(if wide { 96.0 } else { 24.0 });
			ui.vertical(|ui| {
				let lv: [usize; 3] = std::array::from_fn(|k| save::lgc_level(g, k, save::lgc(g, d, k)).0);
				let game_look = lgc_tex.is_some();
				if !game_look {
					head(ui, "L.G.C. Alignment");
				}
				let side = !game_look && ui.available_width() >= 660.0;
				ui.with_layout(if side { egui::Layout::left_to_right(egui::Align::Min) } else { egui::Layout::top_down(egui::Align::Min) }, |ui| {
					match &lgc_tex {
						Some((bg, ch)) => {
							ui.add_space(10.0);
							let room = if wide { ui.clip_rect().max.y - ui.cursor().min.y - 130.0 } else { f32::INFINITY };
							let k = (ui.available_width() / 576.0).min(room / 320.0).clamp(0.45, 1.0);
							lgc_game(ui, bg, ch, lv, save::lgc_max(g), g.lgc_digits, k);
						}
						None => lgc_chart(ui, &p, lv, save::lgc_max(g)),
					}
					ui.add_space(8.0);
					ui.vertical(|ui| {
						ui.add_space(if side { 40.0 } else { 4.0 });
						egui::Grid::new("lgc").num_columns(3).spacing([12.0, 10.0]).show(ui, |ui| {
							for (k, nm) in ["Law", "Gray", "Chaos"].iter().enumerate() {
								let mut v = save::lgc(g, d, k);
								let (lv, _) = save::lgc_level(g, k, v);
								ui.allocate_ui_with_layout(Vec2::new(118.0, 26.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
									ui.set_min_width(118.0);
									ui.label(egui::RichText::new(format!("{nm} LV: {lv}")).font(f_semi(13.5)).color(p.text));
									help_icon(ui, &p, |ui| {
										let s = g.lgc_steps[k];
										let rows: Vec<[String; 2]> = s.iter().enumerate().map(|(i, x)| [format!("LV {}", g.lgc_first + i), x.to_string()]).collect();
										tip_table(ui, &p, &format!("{nm} gauge EXP"), ["Level", "EXP"], &rows, lv.checked_sub(g.lgc_first));
									});
								});
								if ui.add_sized([FW, 26.0], egui::DragValue::new(&mut v).clamp_existing_to_range(false).range(0..=99_999_999).speed(1.0)).changed() {
									save::set_lgc(g, d, k, v);
								}
								let (_, next) = save::lgc_level(g, k, v);
								let txt = match next {
									Some(n) => format!("{n} more to LV {}", lv + 1),
									None => "max".to_string(),
								};
								ui.label(egui::RichText::new(txt).font(f_body(12.5)).color(p.muted));
								ui.end_row();
							}
						});
						ui.add_space(6.0);
						ui.label(egui::RichText::new("The field holds the gauge EXP.").font(f_body(12.0)).color(p.muted));
					});
				});
			});
		});
		});
	}
	fn chr_name(&self, c: u32) -> String {
		match c {
			62 => "Yin (Rixia)".into(),
			_ => self.names.get(&c).map(|n| n.0.clone()).unwrap_or_else(|| format!("#{c}")),
		}
	}

	fn lineup_card(&self, ui: &mut egui::Ui, tex: &Option<egui::TextureHandle>, name: &str, tag: &str, big: bool, selected: bool, target: bool) -> egui::Response {
		let p = self.pal;
		let size = if big { Vec2::new(150.0, 128.0) } else { Vec2::new(140.0, 100.0) };
		let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
		let pt = ui.painter();
		pt.rect_filled(rect, 8, if resp.hovered() || target { p.div2 } else { p.bg });
		let stroke = if selected || target { Stroke::new(2.0, p.accent) } else { Stroke::new(1.0, p.div) };
		pt.rect_stroke(rect, 8, stroke, StrokeKind::Inside);
		let face = if big { 60.0 } else { 44.0 };
		let fr = Rect::from_center_size(Pos2::new(rect.center().x, rect.min.y + 8.0 + face / 2.0), Vec2::splat(face));
		if let Some(t) = tex {
			egui::Image::new(t).paint_at(ui, fr);
		}
		let ny = fr.max.y + 12.0;
		paint_text_clipped(ui, Rect::from_center_size(Pos2::new(rect.center().x, ny), Vec2::new(size.x - 10.0, 18.0)), name, f_body(13.0), p.text, Align2::CENTER_CENTER);
		paint_text_clipped(ui, Rect::from_center_size(Pos2::new(rect.center().x, ny + 16.0), Vec2::new(size.x - 10.0, 16.0)), tag, f_body(11.0), p.muted, Align2::CENTER_CENTER);
		if resp.drag_started() {
			egui::DragAndDrop::set_payload(ui.ctx(), name.to_string());
		}
		resp.on_hover_cursor(egui::CursorIcon::Grab)
	}

	fn lineup(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
		let g = self.g;
		let p = self.pal;
		let (list, have) = {
			let d = self.data.as_ref().unwrap();
			let list = save::lineup(g, d);
			let have: Vec<u32> = g.addable.iter().map(|a| a.0).filter(|c| !list.contains(c)).collect();
			(list, have)
		};
		let unjoined: Vec<u32> = {
			let d = self.data.as_ref().unwrap();
			g.addable.iter().filter(|&&(_, k)| save::u32_at(d, save::chr_at(g, k)) == 0).map(|a| a.0).collect()
		};
		let at = |pos: LPos| match pos {
			LPos::Line(i) => list.get(i).copied(),
			LPos::Avail(j) => have.get(j).copied(),
		};
		if self.lsel.is_some_and(|s| at(s).is_none()) {
			self.lsel = None;
		}
		let faces: HashMap<u32, Option<egui::TextureHandle>> = list.iter().chain(have.iter()).map(|&c| (c, self.face(ctx, c))).collect();
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
			let name = this.chr_name(c);
			let selected = this.lsel == Some(pos);
			let hovered_drop = dragging && ui.rect_contains_pointer(Rect::from_min_size(ui.cursor().min, if big { Vec2::new(150.0, 128.0) } else { Vec2::new(140.0, 100.0) }));
			let r = this.lineup_card(ui, faces.get(&c).unwrap_or(&None), &name, tag, big, selected, hovered_drop);
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
				if c == 0 {
					"always in party"
				} else if unjoined.contains(&c) {
					"hasn't joined"
				} else if g.npc_guests.contains(&c) {
					"guest, AI-controlled"
				} else {
					["in battle", "reserve", "not in party"][zone as usize]
				}
			};
			let z0 = zone(ui, "In battle", "4 slots", &mut |ui| {
				for i in 0..db1::LINEUP_ACTIVE {
					match list.get(i) {
						Some(&c) => card_at(self, ui, LPos::Line(i), c, true, tag(c, 0)),
						None => {
							let (rect, _) = ui.allocate_exact_size(Vec2::new(150.0, 128.0), Sense::hover());
							ui.painter().rect_stroke(rect, 8, Stroke::new(1.0, p.div2), StrokeKind::Inside);
							ui.painter().text(rect.center(), Align2::CENTER_CENTER, "empty", f_body(12.0), p.muted);
						}
					}
				}
			});
			let reserve = list.len().saturating_sub(db1::LINEUP_ACTIVE);
			let z1 = zone(ui, "Reserve", &format!("{reserve}"), &mut |ui| {
				for i in db1::LINEUP_ACTIVE..list.len() {
					card_at(self, ui, LPos::Line(i), list[i], false, tag(list[i], 1));
				}
				if reserve == 0 {
					ui.label(egui::RichText::new("drag here").font(f_body(12.0)).color(p.muted));
				}
			});
			let z2 = zone(ui, "Can be added", "main cast and playable guests", &mut |ui| {
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
					pt.text(r.left_center() + Vec2::new(10.0, 0.0), Align2::LEFT_CENTER, self.chr_name(c), f_body(13.0), p.text);
				}
			}
		}
		let Some((a, b)) = act else { return };
		let mut new = list.clone();
		let mut warn = None;
		match b {
			Err(255) => match self.lsel.take() {
				None => self.lsel = Some(a),
				Some(s) if s == a => {}
				Some(s) => warn = lineup_swap(&mut new, &have, s, a),
			},
			Ok(t) => warn = lineup_swap(&mut new, &have, a, t),
			Err(z) => warn = lineup_move(&mut new, &have, a, z),
		}
		if let Some(w) = warn {
			self.status = format!("✖ {}", if w == VAN_STAYS { "Van is always in the party — he can't be removed" } else { w });
		}
		if b != Err(255) {
			self.lsel = None;
			self.drag_src = None;
		}
		if new != list {
			let added: Vec<(usize, String)> = new
				.iter()
				.filter(|c| !list.contains(c) && unjoined.contains(c))
				.filter_map(|&c| g.addable.iter().find(|a| a.0 == c).map(|a| (a.1, self.chr_name(c))))
				.collect();
			let d = self.data.as_mut().unwrap();
			save::set_lineup(g, d, &new);
			for (k, name) in added {
				save::set_u32(d, save::chr_at(g, k), 1);
				self.status = format!("{name} hasn't joined yet: set to level 1, no weapon or crafts — check in game on a new slot");
			}
		}
	}

	#[allow(dead_code)]
	fn characters(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, equip_mode: bool) {
		let g = self.g;
		let rows: Vec<(usize, Option<u32>)> = {
			let d = self.data.as_ref().unwrap();
			(0..g.char_count)
				.filter(|&k| save::u32_at(d, save::chr_at(g, k)) > 0)
				.map(|k| (k, save::record_chr(g, d, k, &self.owners)))
				.filter(|r| r.1.is_some_and(|c| g.addable.iter().any(|a| a.0 == c)))
				.collect()
		};
		let alias = |c: u32| match c {
			62 => Some("Yin (Rixia)".to_string()),
			_ => None,
		};
		let names: Vec<(String, Option<&str>)> = rows
			.iter()
			.map(|&(k, c)| {
				let n = c.and_then(|c| alias(c).or_else(|| self.names.get(&c).map(|n| n.0.clone()))).unwrap_or_else(|| format!("? (record {})", k + 1));
				let note = match c {
					Some(c) if g.npc_guests.contains(&c) => Some("guest, AI-controlled"),
					None if g.npc_unknown.contains(&k) => Some("Walter / Lucrezia / Kurogane"),
					_ => None,
				};
				(n, note)
			})
			.collect();
		let line = save::lineup(g, self.data.as_ref().unwrap());
		let group = |c: Option<u32>| match c.and_then(|c| line.iter().position(|&x| x == c)) {
			Some(i) if i < db1::LINEUP_ACTIVE => (0, i),
			Some(i) => (1, i),
			None => (2, 0),
		};
		let mut order: Vec<usize> = (0..rows.len()).collect();
		order.sort_by_key(|&n| group(rows[n].1));
		if !rows.iter().any(|r| r.0 == self.chr_sel) {
			self.chr_sel = rows.first().map_or(0, |r| r.0);
		}
		let atlas = self.icons(ctx);
		let (equip, crafts) = {
			let d = self.data.as_ref().unwrap();
			let o = save::chr_at(g, self.chr_sel) + g.equip;
			let equip: Vec<Option<(String, Option<u32>)>> = (0..db1::EQUIP_SLOTS.len())
				.map(|i| {
					let id = save::u32_at(d, o + i * 4) as usize;
					if id == 0 || id >= 0xFFFF {
						return None;
					}
					Some(self.items.iter().find(|it| it.id as usize == id).map_or((format!("#{id}"), None), |it| (it.en.clone(), it.icon)))
				})
				.collect();
			let mut ids = save::crafts(g, d, self.chr_sel);
			ids.sort_by_key(|c| self.skills.get(c).is_none_or(|s| s.kind != 'S'));
			let levels = save::craft_levels(g, d, self.chr_sel);
			let crafts: Vec<(String, u32, String, Option<u32>, Option<u32>, Option<u32>)> = ids
				.into_iter()
				.map(|c| match self.skills.get(&c) {
					Some(s) => {
						let base = levels.iter().filter(|&&(b, _)| b <= c && c - b < 5).max_by_key(|&&(b, _)| b).copied();
						let (cp, pow) = match base {
							Some((b, lv)) => {
								let bs = self.skills.get(&b);
								let at = |v: Option<&Vec<i32>>| v.and_then(|v| v.get(lv as usize - 1)).copied().unwrap_or(0);
								(s.cp as i32 + at(bs.map(|x| &x.cp_lv)), s.power_num as i32 + at(bs.map(|x| &x.power_lv)))
							}
							None => (s.cp as i32, s.power_num as i32),
						};
						let cell = if s.power.is_empty() && s.power_num > 0 { save::cells::power_of(pow.max(0) as u32) } else { save::cells::power(&s.power) };
						(s.en.clone(), cp.max(0) as u32, s.power.clone(), Some(save::cells::craft_kind(s.kind)), cell, s.area)
					}
					None => (format!("#{c}"), 0, String::new(), None, None, None),
				})
				.collect();
			(equip, crafts)
		};
		let screen = if equip_mode { "equip" } else { "status" };
		let nodes = self.ui_nodes(screen);
		let mut texs: HashMap<String, Option<egui::TextureHandle>> = HashMap::new();
		for n in nodes.iter() {
			if let Some(f) = &n.f {
				if !texs.contains_key(f) {
					let t = self.art(ctx, &format!("ui:{f}"));
					texs.insert(f.clone(), t);
				}
			}
		}
		let digits = self.art(ctx, "ui:number_fonts.png");
		let line = save::lineup(g, self.data.as_ref().unwrap());
		let mut slots: Vec<usize> = line.iter().filter_map(|&c| rows.iter().position(|r| r.1 == Some(c))).collect();
		let in_party = slots.len();
		for n in 0..rows.len() {
			if !slots.contains(&n) {
				slots.push(n);
			}
		}
		let slot_models: Vec<Option<u32>> = slots.iter().map(|&n| rows[n].1.and_then(|c| self.models.get(&c).copied())).collect();
		let strips: Vec<Option<egui::TextureHandle>> = slot_models.iter().map(|m| m.and_then(|m| self.art(ctx, &format!("face_c{m:04}")))).collect();
		let model = rows.iter().find(|r| r.0 == self.chr_sel).and_then(|r| r.1).and_then(|c| self.models.get(&c).copied());
		let art_main = model.and_then(|m| self.art(ctx, &format!("mp_c{m:04}")));
		let art_sign = model.and_then(|m| self.art(ctx, &format!("sn_c{m:04}")));
		let art_bp = model.and_then(|m| self.art(ctx, &format!("bp_c{m:04}")));
		let art_sketch = model.and_then(|m| self.art(ctx, &format!("st_c{m:04}")));
		let d = self.data.as_mut().unwrap();
		let full = ui.available_rect_before_wrap();
		let s = (full.width() / 1920.0).min(full.height() / 1080.0);
		let org = Pos2::new(full.center().x - 960.0 * s, full.center().y - 540.0 * s);
		let at = |x: f32, y: f32| org + Vec2::new(x, y) * s;
		let rr = |x0: f32, y0: f32, x1: f32, y1: f32| Rect::from_min_max(at(x0, y0), at(x1, y1));
		let canvas = rr(0.0, 0.0, 1920.0, 1080.0);
		let pt = ui.painter().with_clip_rect(canvas.intersect(full));
		let uv1 = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
		let place = |n: &UiNode, dy: f32| {
			let (ax, ay) = pivot_frac(n.pv);
			Rect::from_min_size(at(n.x - n.w.abs() * n.sc * ax, n.y + dy - n.h.abs() * n.sc * ay), Vec2::new(n.w.abs(), n.h.abs()) * n.sc * s)
		};
		let img = |n: &UiNode, r: Rect, tint: Color32| {
			if let Some(Some(t)) = n.f.as_ref().map(|f| texs.get(f).cloned().flatten()) {
				pt.image(t.id(), r, uv1, tint);
			}
		};
		pt.rect_filled(canvas, 0, Color32::BLACK);
		for n in nodes.iter().filter(|n| n.p.starts_with("status_back_ground") || n.p.starts_with("equip_back_ground")) {
			img(n, place(n, 0.0), tint_of(n));
		}
		if !equip_mode {
			if let Some(t) = &art_sketch {
				pt.image(t.id(), canvas, uv1, Color32::from_white_alpha(200));
			}
		}
		let cx = if equip_mode { 650.0 } else { 960.0 };
		if let Some(t) = &art_main {
			let fig = pt.with_clip_rect(rr(0.0, 0.0, if equip_mode { 905.0 } else { 1920.0 }, 1080.0).intersect(full));
			fig.image(t.id(), rr(cx - 512.0, 56.0, cx + 512.0, 1080.0), Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
		}
		if !equip_mode {
			if let Some(t) = &art_sign {
				pt.image(t.id(), rr(356.0, 438.0, 1380.0, 950.0), uv1, Color32::WHITE);
			}
		}
		let win = Color32::from_rgba_unmultiplied(30, 30, 32, 238);
		let glass = Color32::from_rgba_unmultiplied(22, 22, 24, 190);
		if equip_mode {
			for (y0, y1) in [(155.0, 517.0), (532.0, 670.0), (680.0, 817.0)] {
				pt.rect_filled(rr(905.0, y0, 1430.0, y1), 0, win);
			}
			pt.rect_filled(rr(1532.0, 165.0, 1870.0, 592.0), 0, glass);
		} else {
			pt.rect_filled(rr(1240.0, 158.0, 1782.0, 962.0), 0, glass);
		}
		let scroll = ui.ctx().data(|dd| dd.get_temp::<f32>(ui.id().with("member_scroll"))).unwrap_or(0.0);
		let mut pick = None;
		for (i, &n) in slots.iter().enumerate() {
			let gap = if i >= 8 && i >= in_party { 30.0 } else { 0.0 };
			let (tpl, dy) = if i < 8 { (i, 0.0) } else { (7, (i - 7) as f32 * 74.0 + gap) };
			let dy = dy + if i >= 4 { -scroll } else { 0.0 };
			let pre = format!("member/chr{tpl}/");
			let (k, c) = rows[n];
			let o = save::chr_at(g, k);
			let on = k == self.chr_sel;
			let support = i >= 4;
			let clip = if support { rr(0.0, 572.0, 520.0, 1080.0) } else { canvas }.intersect(full);
			let pt = pt.with_clip_rect(clip);
			let img = |n: &UiNode, r: Rect, tint: Color32| {
				if let Some(Some(t)) = n.f.as_ref().map(|f| texs.get(f).cloned().flatten()) {
					pt.image(t.id(), r, uv1, tint);
				}
			};
			let rat = |cur_i: usize| {
				let max = save::u32_at(d, o + save::stat(g, (cur_i + 1) * 4)).max(1);
				let cur = save::u32_at(d, o + save::stat(g, cur_i * 4));
				(if cur == 99_999 { 1.0 } else { (cur as f32 / max as f32).clamp(0.0, 1.0) }, if cur == 99_999 { max } else { cur })
			};
			let mut base_rect = Rect::NOTHING;
			for nd in nodes.iter().filter(|nd| nd.p.starts_with(&pre) && !nd.p.contains("/lv/")) {
				let r = place(nd, dy);
				let last = nd.p.rsplit('/').next().unwrap_or("");
				match last {
					"base" => {
						base_rect = r;
						let t = if on { Color32::from_rgb(60, 150, 190) } else if support { Color32::from_rgb(52, 74, 100) } else { Color32::from_rgb(46, 92, 140) };
						img(nd, r, t);
					}
					"belt" => img(nd, r, if on { Color32::from_rgb(140, 220, 240) } else { Color32::from_rgb(90, 150, 200) }),
					"face" => {
						if let Some(t) = &strips[i] {
							pt.image(t.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 63.0 / 256.0)), Color32::WHITE);
						}
					}
					"hp_gauge" | "ep_gauge" => {
						let (f, _) = rat(if last == "hp_gauge" { 2 } else { 4 });
						let mut r2 = r;
						r2.set_width(r.width() * f);
						if let Some(Some(t)) = nd.f.as_ref().map(|f| texs.get(f).cloned().flatten()) {
							pt.image(t.id(), r2, Rect::from_min_max(Pos2::ZERO, Pos2::new(f, 1.0)), Color32::WHITE);
						}
					}
					"hp_num" | "ep_num" | "cp_num" if nd.a > 0.0 => {
						let v = match last {
							"hp_num" => rat(2).1,
							"ep_num" => rat(4).1,
							_ => save::u32_at(d, o + save::stat(g, 24)),
						};
						if let Some(t) = &digits {
							draw_digits(&pt, t, nd.uv, nd.ns, &v.to_string(), at(nd.x, nd.y + dy), nd.pv, s * nd.sc, if last == "hp_num" { GAME_GOLD } else { Color32::WHITE });
						}
					}
					"outline" => {
						if on {
							let k = |v: f32| (v.clamp(0.0, 1.0) * 255.0) as u8;
							img(nd, r, Color32::from_rgba_premultiplied(k(nd.c[0]), k(nd.c[1]), k(nd.c[2]), 0));
						}
					}
					_ => {
						if nd.f.is_some() {
							img(nd, r, tint_of(nd));
						}
					}
				}
			}
			if i == in_party && i >= 8 {
				gtext(ui, &pt, Pos2::new(base_rect.max.x - 20.0 * s, base_rect.min.y - 18.0 * s), Align2::RIGHT_CENTER, "NOT IN PARTY", f_semi((17.0 * s).max(9.5)), GAME_CYAN);
			}
			let hit = base_rect.intersect(clip);
			if hit.is_positive() && ui.interact(hit, ui.id().with(("member", i)), Sense::click()).clicked() {
				pick = Some(k);
			}
			let _ = c;
		}
		if ui.rect_contains_pointer(rr(0.0, 560.0, 440.0, 1080.0)) {
			let dw = ui.input(|i| i.smooth_scroll_delta.y) / s.max(0.1);
			if dw != 0.0 {
				let total = (slots.len().saturating_sub(8)) as f32 * 74.0 + if slots.len() > 8 { 30.0 } else { 0.0 };
				let v = (scroll - dw).clamp(0.0, total);
				ui.ctx().data_mut(|dd| dd.insert_temp(ui.id().with("member_scroll"), v));
			}
		}
		if let Some(k) = pick {
			self.chr_sel = k;
		}
		let Some(n) = rows.iter().position(|r| r.0 == self.chr_sel) else { return };
		let o = save::chr_at(g, self.chr_sel);
		let stat_slot = |path: &str| -> Option<usize> {
			let seg: Vec<&str> = path.rsplitn(3, '/').collect();
			let (leaf, owner) = (seg.first().copied().unwrap_or(""), seg.get(1).copied().unwrap_or(""));
			let parent = seg.get(2).map(|p| p.rsplit('/').next().unwrap_or("")).unwrap_or("");
			match (owner, leaf) {
				("lv", "num") => Some(0),
				("hp", "num") => Some(8),
				("hp", "max") => Some(12),
				("ep", "num") => Some(16),
				("ep", "max") => Some(20),
				("cp", "num") => Some(24),
				("cp", "max") => Some(28),
				("attack_root", "num") => Some(if parent == "arts" { 80 } else { 72 }),
				("defence_root", "num") => Some(if parent == "arts" { 84 } else { 76 }),
				("hit", "num") => Some(92),
				("critical", "num") => Some(104),
				("avoid", "num") => Some(96),
				("action", "num") => Some(100),
				("str", "num") => Some(32),
				("def", "num") => Some(36),
				("ats", "num") => Some(40),
				("adf", "num") => Some(44),
				("spd", "num") => Some(48),
				("agl", "num") => Some(52),
				("dex", "num") => Some(56),
				("mov", "num") => Some(60),
				("rng", "num") => Some(64),
				("exp", "num") => Some(4),
				_ => None,
			}
		};
		for nd in nodes.iter() {
			let p = nd.p.as_str();
			if p.starts_with("status_back_ground") || p.starts_with("equip_back_ground") || p.starts_with("member/chr") {
				continue;
			}
			if (equip_mode && p.contains("param_list")) || p.contains("/cursor/bar") || p.ends_with("brueprint/num") || p.ends_with("header/name") || p.contains("item_template") || p.ends_with("next/num") {
				continue;
			}
			let r = place(nd, 0.0);
			if p.ends_with("weapon_print") {
				if let Some(t) = &art_bp {
					let sz = t.size_vec2() * (r.width() / t.size_vec2().x).min(r.height() / t.size_vec2().y);
					pt.image(t.id(), Rect::from_center_size(r.center(), sz), uv1, Color32::WHITE);
				}
				continue;
			}
			if p.contains("/cursor/base") {
				img(nd, r, Color32::from_rgb(70, 70, 72));
				continue;
			}
			if let Some(i) = p.strip_suffix("/name").and_then(|q| q.rsplit('/').next()).and_then(|q| q.strip_prefix("item")).and_then(|q| q.parse::<usize>().ok()) {
				let y = nd.y - 10.0;
				match &equip[i.min(5)] {
					Some((nm, cell)) => {
						if let (Some(tx), Some(cl)) = (&atlas, cell) {
							pt.image(tx.id(), Rect::from_center_size(at(nd.x + 18.0, y), Vec2::splat(34.0 * s)), atlas_uv(tx, *cl), Color32::WHITE);
						}
						gtext(ui, &pt, at(nd.x + 42.0, y), Align2::LEFT_CENTER, nm, f_body((26.0 * s).max(9.5)), GAME_TEXT);
					}
					None => gtext(ui, &pt, at(nd.x + 42.0, y), Align2::LEFT_CENTER, "—", f_body((24.0 * s).max(9.5)), GAME_MUTED),
				}
				continue;
			}
			if p.ends_with("status_list/name/text") {
				gtext(ui, &pt, at(nd.x, nd.y), Align2::LEFT_CENTER, &names[n].0, f_body((nd.fs.unwrap_or(22.0) * 1.1 * s).max(9.5)), Color32::WHITE);
				continue;
			}
			if nd.t == "number_fonts" {
				if let Some(off) = stat_slot(p) {
					let derived = (64..=104).contains(&off);
					let max = match off {
						0 => 150,
						4 => 99_999_999,
						8 | 12 => 99_999,
						16 | 20 => 9_999,
						24 | 28 => 999,
						32..=44 => 9_999,
						60 => 99,
						_ => 999,
					};
					let tint = if off == 8 || off == 12 { GAME_GOLD } else { Color32::WHITE };
					if !equip_mode && !derived {
						let mut v = save::u32_at(d, o + save::stat(g, off));
						let shown_full = off == 8 && v == 99_999;
						if shown_full {
							v = save::u32_at(d, o + 12);
						}
						if game_edit(ui, &pt, &digits, p, nd.uv, nd.ns, at(nd.x, nd.y), nd.pv, s * nd.sc, &mut v, max) {
							save::set_u32(d, o + save::stat(g, off), v);
							if matches!(off, 12 | 20 | 28) {
								let cur = save::u32_at(d, o + save::stat(g, off - 4));
								if cur != 99_999 && cur > v {
									save::set_u32(d, o + save::stat(g, off - 4), v);
								}
							}
						}
					} else if let Some(t) = &digits {
						let mut v = save::u32_at(d, o + save::stat(g, off));
						if off == 8 && v == 99_999 {
							v = save::u32_at(d, o + 12);
						}
						draw_digits(&pt, t, nd.uv, nd.ns, &v.to_string(), at(nd.x, nd.y), nd.pv, s * nd.sc, tint);
					}
					continue;
				}
				if let Some(t) = &digits {
					let ts = t.size_vec2();
					let u = Rect::from_min_size(Pos2::new(nd.uv[0] / ts.x, nd.uv[1] / ts.y), Vec2::new(nd.uv[2] / ts.x, nd.uv[3] / ts.y));
					pt.image(t.id(), r, u, tint_of(nd));
				}
				continue;
			}
			if nd.f.is_some() {
				img(nd, r, tint_of(nd));
			}
		}
		if !equip_mode {
			let tpl: Vec<&UiNode> = nodes.iter().filter(|nd| nd.p.contains("item_template")).collect();
			for (i, (nm, cp, pw, kind, power, area)) in crafts.iter().enumerate().take(8) {
				let dy = i as f32 * 36.0;
				for nd in &tpl {
					let last = nd.p.rsplit('/').next().unwrap_or("");
					match last {
						"name" => {
							if let (Some(tx), Some(cl)) = (&atlas, kind) {
								pt.image(tx.id(), Rect::from_center_size(at(nd.x + 14.0, nd.y + dy - 17.0), Vec2::splat(30.0 * s)), atlas_uv(tx, *cl), Color32::WHITE);
							}
							gtext(ui, &pt, at(nd.x + 38.0, nd.y + dy - 17.0), Align2::LEFT_CENTER, nm, f_body((nd.fs.unwrap_or(20.0) * 1.15 * s).max(9.5)), GAME_TEXT);
						}
						"cp" => img(nd, place(nd, dy), tint_of(nd)),
						"value" => {
							if let Some(t) = &digits {
								draw_digits(&pt, t, nd.uv, nd.ns, &cp.to_string(), at(nd.x, nd.y + dy), nd.pv, s * nd.sc, Color32::WHITE);
							}
						}
						"eff" | "range" => {
							let cell = if last == "eff" { *power } else { *area };
							match (&atlas, cell) {
								(Some(tx), Some(cl)) => {
									pt.image(tx.id(), place(nd, dy), atlas_uv(tx, cl), Color32::WHITE);
								}
								_ if last == "eff" && !pw.is_empty() => gtext(ui, &pt, place(nd, dy).center(), Align2::CENTER_CENTER, pw, f_semi((15.0 * s).max(9.5)), GAME_GOLD),
								_ => {}
							}
						}
						_ => {}
					}
				}
			}
		}
	}
	fn inventory(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
		let g = self.g;
		let p = self.pal;
		let atlas = self.icons(ctx);
		let names: Vec<&str> = g.tabs.iter().copied().chain(["DLC"]).collect();
		let need: f32 = names.iter().map(|t| ui.fonts(|f| f.layout_no_wrap(t.to_string(), f_body(12.0), p.text).size().x) + 28.0).sum();
		let rows: Vec<&[&str]> = if need <= ui.available_width() { vec![&names[..]] } else { let (a, b) = names.split_at(names.len().div_ceil(2)); vec![a, b] };
		let mut start = 0;
		for (k, row) in rows.iter().enumerate() {
			let top = ui.max_rect().min + Vec2::new(0.0, k as f32 * 38.0);
			let mut r = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(top, Vec2::new(ui.max_rect().width(), 32.0))));
			let mut s = self.item_tab.checked_sub(start).filter(|&s| s < row.len()).unwrap_or(usize::MAX);
			if segmented(&mut r, &p, &format!("itab{k}"), row, &mut s, None, 32.0, 8, f_body(12.0), &[]) {
				self.item_tab = start + s;
			}
			start += row.len();
		}
		ui.add_space(42.0 + (rows.len() - 1) as f32 * 38.0);
		ui.horizontal(|ui| {
			ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Search by name").desired_width(260.0));
			ui.add_space(12.0);
			checkbox(ui, &p, &mut self.owned_only, "Owned only", 16.0, f_body(13.0));
		});
		ui.add_space(6.0);
		let dlc_tab = self.item_tab == g.tabs.len();
		let q = self.search.trim().to_lowercase();
		let d = self.data.as_mut().unwrap();
		if dlc_tab {
			let item_of: HashMap<usize, &save::Item> = self.items.iter().map(|it| (it.id, it)).collect();
			let contents = |ui: &mut egui::Ui, pack: &save::Dlc| {
				if pack.items.is_empty() {
					ui.label(&pack.en);
				}
				let total = pack.items.len();
				let cols = if total > 48 { 3 } else if total > 14 { 2 } else { 1 };
				let rows = total.div_ceil(cols);
				egui::Grid::new(("dlc_tip", pack.id)).num_columns(cols).spacing([18.0, 2.0]).show(ui, |ui| {
					for r in 0..rows {
						for c in 0..cols {
							let Some(&(id, n)) = pack.items.get(c * rows + r) else { continue };
							let it = item_of.get(&id);
							let icon = it.and_then(|it| it.icon).or(match id {
								310..=317 => Some(save::cells::SEPITH + (id - 310) as u32),
								_ if it.is_some_and(|it| it.cat == 12) => Some(49),
								_ => None,
							});
							ui.horizontal(|ui| {
								match icon {
									Some(c) => atlas_cell(ui, &atlas, Some(c), 20.0),
									None => {
										ui.add_space(20.0);
									}
								}
								ui.label(egui::RichText::new(format!("{} ×{n}", it.map_or("?", |it| it.en.as_str()))).font(f_body(13.5)));
							});
						}
						ui.end_row();
					}
				});
			};
			let cols = ((ui.available_width() / 300.0).floor() as usize).max(1);
			let cell_w = ui.available_width() / cols as f32;
			let packs: Vec<u32> = self
				.dlcs
				.iter()
				.filter(|pk| q.is_empty() || pk.en.to_lowercase().contains(&q) || pk.ru.to_lowercase().contains(&q))
				.filter(|pk| !self.owned_only || save::dlc_opened(g, d, pk.id))
				.map(|pk| pk.id)
				.collect();
			ui.horizontal(|ui| {
				ui.label(egui::RichText::new("All visible:").font(f_body(13.0)).color(p.muted));
				for n in [0u32, 1] {
					if small_button(ui, &p, &format!("All “{n}”"), None, 26.0, !packs.is_empty(), p.text).clicked() {
						for &id in &packs {
							save::set_dlc_opened(g, d, id, n == 1);
						}
					}
				}
			});
			ui.add_space(6.0);
			egui::ScrollArea::vertical().id_salt(("dlc", g.dir)).auto_shrink([false, false]).show(ui, |ui| {
				for row in packs.chunks(cols) {
					ui.horizontal(|ui| {
						for &id in row {
							let Some(pack) = self.dlcs.iter().find(|pk| pk.id == id) else { continue };
							ui.horizontal(|ui| {
								ui.set_min_height(28.0);
								ui.set_width(cell_w - 8.0);
								atlas_cell(ui, &atlas, Some(6), 24.0);
								let w = (cell_w - 8.0 - 24.0 - 46.0 - 3.0 * ui.spacing().item_spacing.x).max(40.0);
								ui.allocate_ui_with_layout(Vec2::new(w, 24.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
									ui.set_width(w);
									let r = ui.add(egui::Label::new(egui::RichText::new(&pack.en).font(f_body(13.5)).color(p.text)).truncate().sense(Sense::click()));
									click_tip(ui, &r, ("pack", id), |ui| contents(ui, pack));
								});
								let mut n = u32::from(save::dlc_opened(g, d, id));
								if ui.add_sized([46.0, 22.0], egui::DragValue::new(&mut n).clamp_existing_to_range(false).range(0..=1)).on_hover_text("1 — set received (box opened), 0 — not received (if the DLC is installed in Steam, the game will give the box)").changed() {
									save::set_dlc_opened(g, d, id, n == 1);
								}
							});
						}
					});
				}
				ui.add_space(10.0);
				ui.label(egui::RichText::new("Note: without the downloadable content (DLC), character skins and items will not appear in the game. They can be unlocked separately in the «Key» tab.").font(f_body(13.0)).color(p.muted));
			});
			return;
		}
		let shown = |it: &save::Item, d: &[u8]| it.id <= g.item_max_id && (q.is_empty() || it.en.to_lowercase().contains(&q) || it.ru.to_lowercase().contains(&q)) && (!self.owned_only || save::item_count(g, d, it.id) > 0);
		let icon_of = |it: &save::Item| it.icon.or(if it.cat == 12 { Some(49) } else { None });
		let tab = self.item_tab;
		let weapons = g.tabs[tab] == "Weapons";
		let rank = |kind: u8| match self.kinds.get(&kind).map(|s| s.as_str()) {
			Some("Recipes") => -2,
			Some("Event Items") if g.dir == "Daybreak-1" => -1,
			_ => kind as i32,
		};
		let mut groups: Vec<(String, String, Option<u32>, Vec<&save::Item>, i32)> = Vec::new();
		let mut list: Vec<&save::Item> = self.items.iter().filter(|it| save::item_tab(g, it) == Some(tab) && shown(it, d) && (self.show_service || !it.service())).collect();
		if weapons {
			list.sort_by_key(|it| it.id);
		}
		for it in list {
			let (key, title, icon, place) = if weapons && it.cat == 12 {
				let who = self.names.get(&it.owner).map(|n| n.0.clone()).unwrap_or_else(|| format!("#{}", it.owner));
				(format!("{}:w{}", g.dir, it.owner), who, icon_of(it), 0)
			} else if weapons {
				(format!("{}:sub", g.dir), "Sub Parts".to_string(), Some(21), 1)
			} else {
				let title = self.kinds.get(&it.kind).cloned().unwrap_or_else(|| format!("Kind {}", it.kind));
				(format!("{}:{}", g.dir, it.kind), title, None, rank(it.kind))
			};
			match groups.iter_mut().find(|gr| gr.0 == key) {
				Some(gr) => gr.3.push(it),
				None => groups.push((key, title, icon, vec![it], place)),
			}
		}
		groups.sort_by_key(|gr| gr.4);
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
					save::set_item_count(g, d, it.id, n.min(save::item_limit(g, it)));
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
			if fc.add_sized(field.size(), egui::DragValue::new(&mut n).clamp_existing_to_range(false).range(0..=save::item_limit(g, it))).changed() {
				save::set_item_count(g, d, it.id, n);
			}
			ui.painter().line_segment([Pos2::new(field.min.x, rect.min.y), Pos2::new(field.min.x, rect.max.y)], Stroke::new(1.0, p.div));
			ui.painter().rect_stroke(rect, 6, Stroke::new(1.0, p.div), StrokeKind::Inside);
			let inner = rect.shrink2(Vec2::new(6.0, 0.0));
			let icon_r = Rect::from_min_size(Pos2::new(inner.min.x, inner.center().y - 12.0), Vec2::splat(24.0));
			if let (Some(t), Some(cell)) = (&atlas, icon_of(it)) {
				egui::Image::new(t).uv(atlas_uv(t, cell)).paint_at(ui, icon_r);
			}
			let name_r = Rect::from_min_max(Pos2::new(icon_r.max.x + 8.0, inner.min.y), Pos2::new(field.min.x - 8.0, inner.max.y));
			let mut nc = ui.new_child(egui::UiBuilder::new().max_rect(name_r).layout(egui::Layout::left_to_right(egui::Align::Center)));
			let r = nc.add(egui::Label::new(egui::RichText::new(&it.en).font(f_body(13.5)).color(p.text)).truncate().sense(Sense::click()));
			click_tip(&nc, &r, ("item", it.id), |ui| item_tip(ui, &p, it, &atlas));
		};
		let headers = groups.len() > 1;
		egui::ScrollArea::vertical().id_salt(("items", self.item_tab, g.dir)).auto_shrink([false, false]).show(ui, |ui| {
			for (key, title, gicon, its, _) in &groups {
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

fn stats_line(stats: &[i32; 16]) -> String {
	save::ITEM_STATS.iter().zip(stats).filter(|(_, v)| **v != 0).map(|((n, _), v)| format!("{n} {}{}", if *v > 0 { "+" } else { "−" }, v.abs())).collect::<Vec<_>>().join(", ")
}

fn item_tip(ui: &mut egui::Ui, p: &Pal, it: &save::Item, atlas: &Option<egui::TextureHandle>) {
	ui.set_max_width(380.0);
	ui.label(egui::RichText::new(&it.en).font(f_semi(13.5)).color(p.text));
	for line in it.desc_en.trim().lines() {
		ui.horizontal_wrapped(|ui| {
			ui.spacing_mut().item_spacing.x = 0.0;
			let mut rest = line;
			while !rest.is_empty() {
				let (text, icon, tail) = match rest.find("<I").and_then(|a| rest[a..].find('>').map(|b| (a, a + b))) {
					Some((a, b)) => (&rest[..a], rest[a + 2..b].parse::<u32>().ok(), &rest[b + 1..]),
					None => (rest, None, ""),
				};
				if !text.is_empty() {
					ui.label(egui::RichText::new(text).font(f_body(12.5)).color(p.muted));
				}
				if icon.is_some() {
					atlas_cell(ui, atlas, icon, 16.0);
				}
				rest = tail;
			}
		});
	}
	let st = stats_line(&it.stats);
	let flat: String = it.desc_en.chars().filter(|c| !c.is_whitespace()).collect();
	let in_desc = st.split(", ").all(|s| flat.contains(&s.replace(' ', "")));
	if !st.is_empty() && !in_desc {
		ui.label(egui::RichText::new(st).font(f_semi(12.5)).color(p.text));
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
	match image::load_from_memory(include_bytes!("icons/calvard_save.png")) {
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
		let have = [11, 12];
		let mut l = vec![0, 1, 2, 3, 6];
		assert_eq!(lineup_swap(&mut l, &have, LPos::Line(1), LPos::Line(4)), None);
		assert_eq!(l, [0, 6, 2, 3, 1]);
		assert_eq!(lineup_swap(&mut l, &have, LPos::Avail(0), LPos::Line(3)), None);
		assert_eq!(l, [0, 6, 2, 11, 1]);
		assert_eq!(lineup_swap(&mut l, &have, LPos::Line(0), LPos::Avail(1)), Some(VAN_STAYS));
		assert_eq!(lineup_move(&mut l, &have, LPos::Line(0), 2), Some(VAN_STAYS));
		lineup_move(&mut l, &have, LPos::Line(2), 2);
		assert_eq!(l, [0, 6, 11, 1]);
		lineup_move(&mut l, &have, LPos::Avail(1), 0);
		assert_eq!(l, [0, 6, 11, 1], "no free battle slots");
		lineup_move(&mut l, &have, LPos::Avail(1), 1);
		assert_eq!(l, [0, 6, 11, 1, 12]);
		lineup_move(&mut l, &have, LPos::Line(1), 1);
		assert_eq!(l, [0, 11, 1, 12, 6]);
	}
}
