use crate::app_tracker;
use crate::models::*;
use printpdf::*;

fn sfmt(s: u64) -> String {
    let h = s / 3600; let m = (s % 3600) / 60;
    if h > 0 { format!("{}h {:02}m", h, m) } else { format!("{}m", m) }
}

fn or(v: &str, fb: &str) -> String {
    if v.is_empty() || v == "Unknown" || v == "0" { fb.to_string() } else { v.to_string() }
}

fn cap(s: &str, max: usize) -> String {
    if s.len() > max { format!("{}..", &s[..max.saturating_sub(2)]) } else { s.to_string() }
}

const ACCENT: (u8, u8, u8) = (59, 130, 246);
const ACCENT2: (u8, u8, u8) = (139, 92, 246);
const GREEN: (u8, u8, u8) = (34, 197, 94);
const YELLOW: (u8, u8, u8) = (234, 179, 8);
const RED: (u8, u8, u8) = (239, 68, 68);
const MUTED: (u8, u8, u8) = (107, 114, 128);
const DIM: (u8, u8, u8) = (156, 163, 175);
const WHITE: (u8, u8, u8) = (0, 0, 0);

struct Doc {
    doc: PdfDocumentReference,
    ly: PdfLayerReference,
    y: Mm,
    h: IndirectFontRef,
    b: IndirectFontRef,
    pg: u32,
}

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(Rgb::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, None))
}

impl Doc {
    fn new() -> Result<Self, String> {
        let (doc, page, layer) = PdfDocument::new("KruYa Report", Mm(210.0), Mm(297.0), "Main");
        let ly = doc.get_page(page).get_layer(layer);
        let h = doc.add_builtin_font(BuiltinFont::Helvetica).map_err(|e| e.to_string())?;
        let b = doc.add_builtin_font(BuiltinFont::HelveticaBold).map_err(|e| e.to_string())?;
        Ok(Doc { doc, ly, y: Mm(270.0), h, b, pg: 1 })
    }

    fn nxt(&mut self) {
        self.pg += 1;
        let (pid, lidx) = self.doc.add_page(Mm(210.0), Mm(297.0), format!("p{}", self.pg));
        self.ly = self.doc.get_page(pid).get_layer(lidx);
        self.y = Mm(270.0);
    }

    fn nd(&mut self, mm: f32) {
        if self.y.0 <= 16.0 + mm { self.nxt(); }
    }

    fn setc(&self, c: (u8, u8, u8)) {
        self.ly.set_fill_color(rgb(c.0, c.1, c.2));
    }

    fn wc(&self, s: &str, sz: f32, x: f32, c: (u8, u8, u8)) {
        self.setc(c);
        self.ly.use_text(s.to_string(), sz, Mm(x), self.y, &self.h);
        self.setc(WHITE);
    }

    fn wbc(&self, s: &str, sz: f32, x: f32, c: (u8, u8, u8)) {
        self.setc(c);
        self.ly.use_text(s.to_string(), sz, Mm(x), self.y, &self.b);
        self.setc(WHITE);
    }

    fn w(&self, s: &str, sz: f32, x: f32) {
        self.ly.use_text(s.to_string(), sz, Mm(x), self.y, &self.h);
    }

    fn wb(&self, s: &str, sz: f32, x: f32) {
        self.ly.use_text(s.to_string(), sz, Mm(x), self.y, &self.b);
    }

    fn hr(&mut self) {
        self.y.0 -= 2.0;
        self.nd(2.0);
        self.wc(&"\u{2500} ".repeat(42), 6.0, 12.0, DIM);
        self.y.0 -= 3.0;
    }

    fn row(&mut self, lbl: &str, val: &str) {
        self.nd(6.0);
        self.wbc(lbl, 9.0, 12.0, ACCENT);
        self.w(val, 9.0, 45.0);
        self.y.0 -= 6.0;
    }

    fn sec(&mut self, t: &str) {
        self.y.0 -= 5.0;
        self.nd(18.0);
        self.y.0 -= 3.0;
        self.hr();
        self.wbc(t, 15.0, 12.0, ACCENT2);
        self.y.0 -= 7.0;
    }

    fn tbl(&mut self, cols: &[(&str, f32)]) {
        self.nd(5.0);
        for (l, x) in cols { self.wc(l, 8.0, *x, ACCENT); }
        self.y.0 -= 5.0;
    }

    fn tr(&mut self, vals: &[(&str, f32)]) {
        if self.y.0 <= 18.0 { self.nxt(); }
        self.nd(4.5);
        for (v, x) in vals { self.w(v, 8.0, *x); }
        self.y.0 -= 4.5;
    }
}

fn health_color(score: u8) -> (u8, u8, u8) {
    if score >= 80 { GREEN } else if score >= 50 { YELLOW } else { RED }
}

pub fn generate_pdf(
    path: &str,
    snapshot: &SystemSnapshot,
    analysis: &AnalysisOutput,
    app_usage: &[app_tracker::AppUsage],
    startup_programs: &[StartupProgram],
    stats: &ReportStats,
) -> Result<(), String> {
    let mut d = Doc::new()?;
    let si = &snapshot.sys_info;
    let sm = &snapshot.system;

    // ══ HEADER ══
    d.wbc("KruYa", 30.0, 12.0, ACCENT);
    d.y.0 -= 1.0;
    d.wbc("KruYa System Monitor Report", 14.0, 12.0, ACCENT2);
    d.y.0 -= 3.0;
    d.wc(&" \u{2500}".repeat(90), 6.0, 12.0, DIM);
    d.y.0 -= 4.0;
    d.wc(&format!("{}  |  {}", snapshot.timestamp, si.hostname), 8.0, 12.0, MUTED);
    d.y.0 -= 14.0;

    // ══ SYSTEM ══
    d.sec("System");
    let tg = sm.memory_total_mb / 1024;
    let ug = sm.memory_used_mb / 1024;
    let ag = sm.memory_available_mb / 1024;
    let cg = tg.saturating_sub(ug + ag);
    let cpu_extra = format!("{} cores ({}p)  {} max  {}", si.cpu_cores, si.cpu_physical_cores,
        if si.cpu_max_frequency_mhz >= 1000 { format!("{:.1} GHz", si.cpu_max_frequency_mhz as f64 / 1000.0) } else { format!("{} MHz", si.cpu_max_frequency_mhz) },
        or(&si.cpu_architecture, "x64"));
    let ram_conf = if si.ram_speed_mhz > 0 && !si.ram_form_factor.is_empty() && si.ram_form_factor != "Unknown" {
        format!("{} MHz {}  {} / {} slots", si.ram_speed_mhz, si.ram_form_factor, si.ram_slots_used, si.ram_slots_total)
    } else {
        format!("{} / {} slots used", si.ram_slots_used, si.ram_slots_total)
    };

    d.row("OS", &format!("{} {}  build {} ({})", si.os_name, si.os_edition, si.os_build, si.os_architecture));
    d.row("Installed", &or(&si.os_install_date, "N/A"));
    d.row("Motherboard", &format!("{} {}", or(&si.motherboard_manufacturer, "N/A"), or(&si.motherboard_model, "")));
    d.row("System", &format!("{} {}", or(&si.system_manufacturer, "N/A"), or(&si.system_model, "")));
    d.row("BIOS", &format!("{} {} ({})", si.bios_vendor, si.bios_version, or(&si.bios_date, "N/A")));
    d.row("CPU", &format!("{}  {}", si.cpu_brand, cpu_extra));
    let per_core = sm.per_core_usage.iter().map(|c| format!("{:.0}%", c)).collect::<Vec<_>>().join(" ");
    d.row("CPU Load", &format!("{:.1}%  per core: {}", sm.cpu_total_usage, cap(&per_core, 200)));
    d.row("RAM", &format!("{} GB / {} GB ({}%)  cached {}  free {}", ug, tg, sm.memory_total_usage, format!("{} GB", cg), format!("{} GB", ag)));
    d.row("RAM Config", &ram_conf);
    d.row("Disk", &format!("{} GB / {} GB ({}%)", sm.disk_used_gb, sm.disk_total_gb, sm.disk_usage_percent));
    d.row("Swap", &format!("{} MB / {} MB", sm.swap_used_mb, sm.swap_total_mb));
    d.row("Uptime", &sfmt(sm.uptime_seconds));
    d.row("Processes", &format!("{} run / {} total", sm.running_processes, sm.total_processes));
    if sm.battery_percent >= 0 {
        let mut bs = format!("{}%", sm.battery_percent);
        if sm.ac_line_status == 1 { bs += " (Charging)"; }
        else if sm.ac_line_status == 0 { bs += " (On Battery)"; }
        d.row("Battery", &bs);
    }
    let hc = health_color(analysis.system_health.severity_score);
    d.wbc("Health", 9.0, 12.0, ACCENT);
    d.wc(&format!("{} (score {}/100)", analysis.system_health.status, analysis.system_health.severity_score), 9.0, 45.0, hc);
    d.y.0 -= 6.0;

    // ══ PERFORMANCE ══
    d.sec(&format!("Performance ({} samples)", stats.samples));
    d.row("CPU", &format!("Avg {:.1}%  Peak {:.1}%  Now {:.1}%", stats.avg_cpu, stats.max_cpu, sm.cpu_total_usage));
    d.row("RAM", &format!("Avg {}%  Peak {}%  Now {}%", stats.avg_mem, stats.max_mem, sm.memory_total_usage));
    if stats.avg_temp > 0.0 {
        d.row("Temperature", &format!("Avg {:.1}C  Peak {:.1}C  Now {:.1}C", stats.avg_temp, stats.max_temp, sm.temperature_celsius));
    }

    // ══ ANALYSIS ══
    d.sec("Analysis");
    d.wc(&cap(&analysis.summary, 500), 8.0, 12.0, MUTED);
    d.y.0 -= 5.0;
    d.row("Bottleneck", &analysis.resource_analysis.bottleneck);
    d.row("CPU Trend", &analysis.trend_analysis.cpu_trend);
    d.row("Memory Trend", &analysis.trend_analysis.memory_trend);
    d.row("Focus", &format!("{} \u{2013} {}", analysis.focus_analysis.primary_app, cap(&analysis.focus_analysis.assessment, 200)));
    if !analysis.anomalies.is_empty() {
        d.y.0 -= 2.0;
        d.wbc("Anomalies", 9.0, 12.0, YELLOW);
        d.y.0 -= 4.0;
        for a in &analysis.anomalies {
            if d.y.0 <= 18.0 { d.nxt(); }
            d.nd(4.5);
            let ac = if a.severity == "high" || a.severity == "critical" { RED } else { YELLOW };
            d.wc(&format!("[{}] {}: {}", a.severity, a.anomaly_type, cap(&a.detail, 250)), 7.5, 12.0, ac);
            d.y.0 -= 4.5;
        }
    }
    if !analysis.alerts.is_empty() {
        for a in &analysis.alerts {
            if d.y.0 <= 18.0 { d.nxt(); }
            d.nd(4.5);
            let ac = if a.severity == "high" || a.severity == "critical" { RED } else { YELLOW };
            d.wc(&format!("[{}] {}: {}", a.severity, a.alert_type, cap(&a.message, 250)), 7.5, 12.0, ac);
            d.y.0 -= 4.5;
        }
    }

    // ══ PROCESSES ══
    d.sec("Top Processes");
    d.tbl(&[("Process", 12.0), ("CPU", 100.0), ("Memory", 125.0), ("Threads", 160.0)]);
    for p in &snapshot.processes {
        d.tr(&[(&cap(&p.name, 50), 12.0), (&format!("{:.1}%", p.cpu_percent), 100.0), (&format!("{} MB", p.memory_mb), 125.0), (&p.threads.to_string(), 160.0)]);
    }

    // ══ APP USAGE ══
    d.sec("App Usage");
    d.tbl(&[("Application", 12.0), ("Time", 100.0), ("Last Seen", 130.0)]);
    for u in app_usage {
        d.tr(&[(&cap(&u.name, 40), 12.0), (&sfmt(u.total_seconds), 100.0), (&u.last_seen, 130.0)]);
    }

    // ══ STARTUP ══
    d.sec("Startup Programs");
    for s in startup_programs {
        if d.y.0 <= 24.0 { d.nxt(); }
        d.nd(10.0);
        d.wbc(&s.name, 9.0, 12.0, ACCENT);
        d.wc(&format!("Enabled: {}", if s.enabled { "Yes" } else { "No" }), 8.0, 130.0, if s.enabled { GREEN } else { MUTED });
        d.y.0 -= 4.5;
        d.wc(&cap(&s.command, 350), 7.5, 16.0, DIM);
        d.y.0 -= 4.0;
        d.wc(&format!("Source: {}", cap(&s.location, 200)), 7.5, 16.0, MUTED);
        d.y.0 -= 6.0;
    }

    // ══ RECOMMENDATIONS ══
    d.sec("Recommendations");
    for r in &analysis.recommendations {
        if d.y.0 <= 18.0 { d.nxt(); }
        d.nd(5.0);
        d.wc(&format!("\u{2022} {}", cap(r, 380)), 8.0, 12.0, GREEN);
        d.y.0 -= 5.0;
    }

    // ══ DISPLAYS ══
    d.sec("Displays");
    d.tbl(&[("Name", 12.0), ("Resolution", 70.0), ("Refresh", 130.0), ("Primary", 160.0)]);
    for dis in &snapshot.displays {
        d.tr(&[(&cap(&dis.name, 30), 12.0), (&format!("{}x{}", dis.resolution_width, dis.resolution_height), 70.0), (&format!("{} Hz", dis.refresh_rate), 130.0), (if dis.is_primary { "Yes" } else { "" }, 160.0)]);
    }

    // ══ DRIVES ══
    d.sec("Drives");
    d.tbl(&[("Mount", 12.0), ("Type", 55.0), ("Used", 95.0), ("Total", 125.0), ("Usage", 155.0)]);
    for dr in &snapshot.drives {
        let lb = if dr.is_removable { format!("{} [Removable]", dr.mount_point) } else { dr.mount_point.clone() };
        d.tr(&[(&cap(&lb, 20), 12.0), (&dr.filesystem_type, 55.0), (&format!("{} GB", dr.used_gb), 95.0), (&format!("{} GB", dr.total_gb), 125.0), (&format!("{}%", dr.usage_percent), 155.0)]);
    }

    // ══ GPU ══
    if !snapshot.gpu.name.is_empty() && snapshot.gpu.name != "Unknown" {
        d.sec("Graphics");
        d.row("GPU", &snapshot.gpu.name);
        if snapshot.gpu.memory_mb > 0 { d.row("VRAM", &format!("{} MB", snapshot.gpu.memory_mb)); }
        d.row("Driver", &snapshot.gpu.driver_version);
        d.row("Type", &snapshot.gpu.gpu_type);
    }

    // ══ NETWORK ══
    d.sec("Network");
    d.tbl(&[("Interface", 12.0), ("Received", 80.0), ("Sent", 135.0)]);
    for inf in &snapshot.interfaces {
        let rx = if inf.total_received >= 1_000_000_000 { format!("{:.1} GB", inf.total_received as f64 / 1e9) }
                 else if inf.total_received >= 1_000_000 { format!("{:.1} MB", inf.total_received as f64 / 1e6) }
                 else { format!("{} B", inf.total_received) };
        let tx = if inf.total_transmitted >= 1_000_000_000 { format!("{:.1} GB", inf.total_transmitted as f64 / 1e9) }
                 else if inf.total_transmitted >= 1_000_000 { format!("{:.1} MB", inf.total_transmitted as f64 / 1e6) }
                 else { format!("{} B", inf.total_transmitted) };
        d.tr(&[(&cap(&inf.name, 30), 12.0), (&rx, 80.0), (&tx, 135.0)]);
    }

    // ══ FOOTER ══
    d.y.0 -= 10.0;
    d.nd(6.0);
    d.wc(&" \u{2500}".repeat(90), 6.0, 12.0, DIM);
    d.y.0 -= 3.0;
    d.wc("KruYa  v0.1.0-alpha.1  |  System Monitor", 7.0, 12.0, MUTED);

    d.doc.save(&mut std::io::BufWriter::new(
        std::fs::File::create(path).map_err(|e| e.to_string())?,
    )).map_err(|e| e.to_string())?;

    Ok(())
}

