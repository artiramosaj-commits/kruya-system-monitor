use crate::models::*;

pub struct AnalyticsEngine {
    history: Vec<SystemSnapshot>,
    alerted_types: std::collections::HashSet<String>,
}

impl AnalyticsEngine {
    pub fn new() -> Self {
        AnalyticsEngine {
            history: Vec::new(),
            alerted_types: std::collections::HashSet::new(),
        }
    }

    pub fn add_snapshot(&mut self, snap: SystemSnapshot) {
        self.history.push(snap);
        if self.history.len() > 100 {
            self.history.remove(0);
        }
    }

    pub fn analyze(&mut self, snap: &SystemSnapshot) -> AnalysisOutput {
        let previous = self.history.last().cloned();

        let anomalies = self.detect_anomalies(snap, previous.as_ref());
        let severity = self.compute_severity(snap, &anomalies);
        let observations = self.generate_observations(snap, previous.as_ref());
        let recommendations = self.generate_recommendations(snap, &anomalies, severity);
        let top_procs = self.get_top_processes(snap);
        let alerts = self.generate_alerts(&anomalies, severity);

        let resource = self.analyze_resources(snap);
        let focus = self.analyze_focus(snap);
        let trend = self.analyze_trends(snap, previous.as_ref());

        let perf_insights: Vec<String> = observations
            .iter()
            .filter(|o| {
                o.contains("CPU")
                    || o.contains("memory")
                    || o.contains("disk")
                    || o.contains("GPU")
                    || o.contains("battery")
                    || o.contains("uptime")
                    || o.contains("process")
            })
            .cloned()
            .collect();

        let prod_insights: Vec<String> = observations
            .iter()
            .filter(|o| o.contains("focus") || o.contains("switching") || o.contains("workflow"))
            .cloned()
            .collect();

        // Build severity status
        let status = if severity <= 20 {
            "Normal"
        } else if severity <= 40 {
            "Minor Inefficiencies"
        } else if severity <= 60 {
            "Moderate Concerns"
        } else if severity <= 80 {
            "Significant Issues"
        } else {
            "Critical Issues"
        };

        // Summary
        let summary = self.generate_summary(snap, &anomalies, severity, previous.as_ref());

        AnalysisOutput {
            summary,
            system_health: HealthStatus {
                status: status.to_string(),
                severity_score: severity,
            },
            performance: perf_insights,
            productivity: prod_insights,
            anomalies,
            recommendations,
            observations,
            top_processes: top_procs,
            focus_analysis: focus,
            resource_analysis: resource,
            trend_analysis: trend,
            alerts,
        }
    }

    fn compute_severity(&self, snap: &SystemSnapshot, anomalies: &[Anomaly]) -> u8 {
        let mut score: f32 = 0.0;

        // Memory pressure (0-30 points)
        let mem = snap.system.memory_total_usage;
        if mem > 90 {
            score += 30.0;
        } else if mem > 80 {
            score += 20.0;
        } else if mem > 70 {
            score += 10.0;
        }

        // CPU saturation (0-20 points)
        let cpu = snap.system.cpu_total_usage;
        if cpu > 90.0 {
            score += 20.0;
        } else if cpu > 75.0 {
            score += 15.0;
        } else if cpu > 50.0 {
            score += 8.0;
        } else if cpu > 30.0 {
            score += 4.0;
        }

        // Disk pressure (0-10)
        if snap.system.disk_usage_percent > 95 {
            score += 10.0;
        } else if snap.system.disk_usage_percent > 85 {
            score += 5.0;
        }

        // Uptime (0-10)
        let uptime_days = snap.system.uptime_seconds / 86400;
        if uptime_days > 14 {
            score += 10.0;
        } else if uptime_days > 7 {
            score += 6.0;
        } else if uptime_days > 3 {
            score += 3.0;
        }

        // Anomalies (0-20)
        for a in anomalies {
            match a.severity.as_str() {
                "critical" => score += 20.0,
                "high" => score += 12.0,
                "medium" => score += 6.0,
                "low" => score += 3.0,
                _ => {}
            }
        }

        // Battery (0-10)
        let bat = snap.system.battery_percent;
        if (0..10).contains(&bat) {
            score += 10.0;
        } else if (0..20).contains(&bat) {
            score += 6.0;
        }

        score.clamp(0.0, 100.0) as u8
    }

    fn detect_anomalies(
        &self,
        snap: &SystemSnapshot,
        previous: Option<&SystemSnapshot>,
    ) -> Vec<Anomaly> {
        let mut anomalies = Vec::new();

        // Memory threshold
        if snap.system.memory_total_usage > 90 {
            anomalies.push(Anomaly {
                anomaly_type: "Memory Saturation".into(),
                severity: "high".into(),
                detail: format!(
                    "Memory at {}% with {}MB used. System is critically low on available RAM.",
                    snap.system.memory_total_usage, snap.system.memory_used_mb
                ),
            });
        } else if snap.system.memory_total_usage > 80 {
            anomalies.push(Anomaly {
                anomaly_type: "Memory Pressure".into(),
                severity: "medium".into(),
                detail: format!(
                    "Memory at {}%. System is memory-constrained.",
                    snap.system.memory_total_usage
                ),
            });
        }

        // CPU threshold
        if snap.system.cpu_total_usage > 90.0 {
            anomalies.push(Anomaly {
                anomaly_type: "CPU Saturation".into(),
                severity: "high".into(),
                detail: format!(
                    "CPU at {}% — system is fully saturated.",
                    snap.system.cpu_total_usage
                ),
            });
        } else if snap.system.cpu_total_usage > 75.0 {
            anomalies.push(Anomaly {
                anomaly_type: "High CPU Load".into(),
                severity: "medium".into(),
                detail: format!(
                    "CPU at {}% — sustained elevated load detected.",
                    snap.system.cpu_total_usage
                ),
            });
        }

        // Extended uptime
        let uptime_days = snap.system.uptime_seconds / 86400;
        if uptime_days > 14 {
            anomalies.push(Anomaly {
                anomaly_type: "Extended Uptime".into(),
                severity: "medium".into(),
                detail: format!(
                    "System has been running for {} days. Long uptime contributes to memory fragmentation and accumulated cache.",
                    uptime_days
                ),
            });
        } else if uptime_days > 7 {
            anomalies.push(Anomaly {
                anomaly_type: "Extended Uptime".into(),
                severity: "low".into(),
                detail: format!(
                    "System running for {} days. Consider restarting for system hygiene.",
                    uptime_days
                ),
            });
        }

        // Process-level anomalies
        for proc in &snap.processes {
            if proc.memory_mb > 3000 {
                anomalies.push(Anomaly {
                    anomaly_type: "Process Memory High".into(),
                    severity: "medium".into(),
                    detail: format!(
                        "{} at {}MB — unusually high memory consumption.",
                        proc.name, proc.memory_mb
                    ),
                });
            }
            if proc.cpu_percent > 50.0 {
                anomalies.push(Anomaly {
                    anomaly_type: "Process CPU High".into(),
                    severity: "medium".into(),
                    detail: format!(
                        "{} at {:.1}% CPU — sustained high compute usage.",
                        proc.name, proc.cpu_percent
                    ),
                });
            }
        }

        // Delta anomalies if we have history
        if let Some(prev) = previous {
            let mem_delta = snap.system.memory_total_usage as i16 - prev.system.memory_total_usage as i16;
            if mem_delta > 5 {
                anomalies.push(Anomaly {
                    anomaly_type: "Memory Increase".into(),
                    severity: "medium".into(),
                    detail: format!(
                        "Memory increased {}% since last snapshot. Investigate new process or leak.",
                        mem_delta
                    ),
                });
            }
        }

        anomalies
    }

    fn generate_observations(
        &self,
        snap: &SystemSnapshot,
        previous: Option<&SystemSnapshot>,
    ) -> Vec<String> {
        let mut obs = Vec::new();
        let mem = snap.system.memory_total_usage;
        let cpu = snap.system.cpu_total_usage;
        let uptime_days = snap.system.uptime_seconds / 86400;
        let uptime_hours = snap.system.uptime_seconds / 3600;

        obs.push(format!(
            "Memory at {}% — {}MB used of {}MB total.",
            mem, snap.system.memory_used_mb, snap.system.memory_total_mb
        ));

        obs.push(format!(
            "CPU at {:.1}% across {} processes.",
            cpu, snap.system.total_processes
        ));

        if uptime_days > 0 {
            obs.push(format!(
                "System uptime: {} days {} hours.",
                uptime_days, uptime_hours % 24
            ));
        } else {
            obs.push(format!(
                "System uptime: {} hours.",
                uptime_hours
            ));
        }

        if snap.system.disk_usage_percent > 0 {
            obs.push(format!(
                "Disk at {}% — {}GB used of {}GB total.",
                snap.system.disk_usage_percent,
                snap.system.disk_used_gb,
                snap.system.disk_total_gb
            ));
        }

        if snap.system.battery_percent >= 0 {
            obs.push(format!("Battery at {}%.", snap.system.battery_percent));
        }

        // Top 3 processes
        let top3: Vec<&str> = snap.processes.iter().take(3).map(|p| p.name.as_str()).collect();
        if !top3.is_empty() {
            obs.push(format!(
                "Top processes by memory: {}.",
                top3.join(", ")
            ));
        }

        // Delta observations
        if let Some(prev) = previous {
            let delta_mem = mem as i16 - prev.system.memory_total_usage as i16;
            let delta_cpu = (cpu - prev.system.cpu_total_usage) as i16;
            if delta_mem.abs() > 2 {
                let dir = if delta_mem > 0 { "increased" } else { "decreased" };
                obs.push(format!("Memory {} by {}% since last snapshot.", dir, delta_mem.abs()));
            }
            if delta_cpu.abs() > 5 {
                let dir = if delta_cpu > 0 { "increased" } else { "decreased" };
                obs.push(format!("CPU {} by {}% since last snapshot.", dir, delta_cpu.abs()));
            }
        }

        obs
    }

    fn generate_recommendations(
        &self,
        snap: &SystemSnapshot,
        anomalies: &[Anomaly],
        severity: u8,
    ) -> Vec<String> {
        let mut recs = Vec::new();
        let mem = snap.system.memory_total_usage;
        let uptime_days = snap.system.uptime_seconds / 86400;

        if mem > 80 {
            recs.push("Restart the system to reclaim accumulated memory from long uptime.".into());
            recs.push("Close unused browser tabs and applications to reduce memory pressure.".into());
        }

        if uptime_days > 7 {
            recs.push("A restart is recommended — uptime exceeds 7 days and accumulated state may degrade performance.".into());
        }

        for a in anomalies {
            match a.anomaly_type.as_str() {
                "Process Memory High" => {
                    recs.push(format!(
                        "Review {} memory usage — consider restarting the process if it has been running for hours.",
                        a.detail.split(" at ").next().unwrap_or("the process")
                    ));
                }
                "Process CPU High" => {
                    recs.push(format!(
                        "Investigate high CPU usage from {} — check if it's performing expected work.",
                        a.detail.split(" at ").next().unwrap_or("the process")
                    ));
                }
                _ => {}
            }
        }

        if severity > 60 {
            recs.push("Immediate attention recommended — system metrics indicate significant resource pressure.".into());
        }

        recs
    }

    fn get_top_processes(&self, snap: &SystemSnapshot) -> Vec<TopProcess> {
        snap.processes
            .iter()
            .take(5)
            .map(|p| {
                let assessment = if p.cpu_percent > 30.0 {
                    "High CPU consumer"
                } else if p.memory_mb > 2000 {
                    "High memory consumer"
                } else {
                    "Normal operation"
                };
                TopProcess {
                    name: p.name.clone(),
                    cpu_percent: p.cpu_percent,
                    memory_mb: p.memory_mb,
                    threads: p.threads,
                    assessment: assessment.into(),
                }
            })
            .collect()
    }

    fn analyze_resources(&self, snap: &SystemSnapshot) -> ResourceAnalysis {
        let mem = snap.system.memory_total_usage;
        let cpu = snap.system.cpu_total_usage;

        let bottleneck = if mem > 80 {
            "Memory is the likely primary bottleneck."
        } else if cpu > 75.0 {
            "CPU saturation detected."
        } else {
            "No immediate resource bottleneck detected."
        };

        let assessment = format!(
            "CPU at {:.1}%, Memory at {}%. {}",
            cpu, mem, bottleneck
        );

        ResourceAnalysis {
            cpu,
            memory: mem,
            bottleneck: bottleneck.into(),
            assessment,
        }
    }

    fn analyze_focus(&self, snap: &SystemSnapshot) -> FocusAnalysis {
        if snap.foreground.name != "unknown" && snap.foreground.pid != 0 {
            FocusAnalysis {
                primary_app: snap.foreground.name.clone(),
                assessment: format!(
                    "Foreground application: {}. Single-application focus detected.",
                    snap.foreground.name
                ),
            }
        } else {
            FocusAnalysis {
                primary_app: "unknown".into(),
                assessment: "Foreground tracking not available on this platform.".into(),
            }
        }
    }

    fn analyze_trends(
        &self,
        snap: &SystemSnapshot,
        previous: Option<&SystemSnapshot>,
    ) -> TrendAnalysis {
        let cpu_trend = if let Some(prev) = previous {
            let d = snap.system.cpu_total_usage - prev.system.cpu_total_usage;
            if d > 5.0 {
                "Increasing sharply"
            } else if d > 1.0 {
                "Slightly increasing"
            } else if d < -5.0 {
                "Decreasing sharply"
            } else if d < -1.0 {
                "Slightly decreasing"
            } else {
                "Stable"
            }
        } else {
            "No baseline available"
        };

        let mem_trend = if let Some(prev) = previous {
            let d = snap.system.memory_total_usage as i16 - prev.system.memory_total_usage as i16;
            if d > 5 {
                "Increasing sharply"
            } else if d > 2 {
                "Slightly increasing"
            } else if d < -5 {
                "Decreasing sharply"
            } else if d < -2 {
                "Slightly decreasing"
            } else {
                "Stable"
            }
        } else {
            "No baseline available"
        };

        TrendAnalysis {
            cpu_trend: cpu_trend.into(),
            memory_trend: mem_trend.into(),
            assessment: format!(
                "CPU is {}. Memory is {}. {} snapshots in history.",
                cpu_trend.to_lowercase(),
                mem_trend.to_lowercase(),
                self.history.len()
            ),
        }
    }

    fn generate_alerts(
        &mut self,
        anomalies: &[Anomaly],
        severity: u8,
    ) -> Vec<Alert> {
        let mut alerts = Vec::new();

        for a in anomalies {
            let key = format!("anomaly:{}", a.anomaly_type);
            if !self.alerted_types.insert(key) {
                continue; // already alerted this type
            }
            let sev = match a.severity.as_str() {
                "critical" => "critical",
                "high" => "warning",
                "medium" => "warning",
                _ => "info",
            };
            alerts.push(Alert {
                alert_type: a.anomaly_type.clone(),
                severity: sev.into(),
                message: a.detail.clone(),
            });
        }

        if severity > 60 {
            let key = "severity:high".to_string();
            if self.alerted_types.insert(key) {
                alerts.push(Alert {
                    alert_type: "High Severity Score".into(),
                    severity: "warning".into(),
                    message: format!(
                        "System severity score is {}/100. Immediate attention recommended.",
                        severity
                    ),
                });
            }
        }

        alerts
    }

    fn generate_summary(
        &self,
        snap: &SystemSnapshot,
        anomalies: &[Anomaly],
        severity: u8,
        previous: Option<&SystemSnapshot>,
    ) -> String {
        let mem = snap.system.memory_total_usage;
        let cpu = snap.system.cpu_total_usage;
        let uptime_days = snap.system.uptime_seconds / 86400;

        let state = if mem > 85 {
            "critical memory pressure"
        } else if mem > 75 {
            "elevated memory pressure"
        } else {
            "normal memory levels"
        };

        let top = snap.processes.first().map(|p| p.name.as_str()).unwrap_or("unknown");

        let anomaly_part = if !anomalies.is_empty() {
            let critical_count = anomalies.iter().filter(|a| a.severity == "high" || a.severity == "critical").count();
            let total = anomalies.len();
            if critical_count > 0 {
                format!(" {} critical and {} total anomalies detected.", critical_count, total)
            } else {
                format!(" {} anomaly(ies) detected.", total)
            }
        } else {
            String::new()
        };

        let delta = if let Some(prev) = previous {
            let d = snap.system.memory_total_usage as i16 - prev.system.memory_total_usage as i16;
            if d.abs() > 0 {
                let dir = if d > 0 { "up" } else { "down" };
                format!(" Memory trending {} from previous snapshot.", dir)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let uptime_part = if uptime_days > 7 {
            format!(" System has been running for {} days.", uptime_days)
        } else {
            String::new()
        };

        format!(
            "System at {}% CPU and {}% memory ({}) | Top consumer: {}.{}{}{} Severity score: {}/100.",
            cpu as u8, mem, state, top, anomaly_part, delta, uptime_part, severity
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_snap(
        cpu: f32,
        mem_pct: u8,
        disk_pct: u8,
        uptime_secs: u64,
        battery: i8,
        temp: f32,
    ) -> SystemSnapshot {
        SystemSnapshot {
            timestamp: "2026-01-01T00:00:00.000Z".into(),
            system: SystemMetrics {
                cpu_total_usage: cpu,
                per_core_usage: vec![cpu],
                cpu_frequency_mhz: vec![2400],
                cpu_brand: "Test CPU".into(),
                memory_total_mb: 16384,
                memory_used_mb: (mem_pct as f64 / 100.0 * 16384.0) as u64,
                memory_available_mb: 16384 - (mem_pct as f64 / 100.0 * 16384.0) as u64,
                memory_total_usage: mem_pct,
                swap_total_mb: 4096,
                swap_used_mb: 0,
                disk_total_gb: 500,
                disk_used_gb: (disk_pct as f64 / 100.0 * 500.0) as u64,
                disk_usage_percent: disk_pct,
                disk_read_bytes: 0,
                disk_write_bytes: 0,
                network_down_bytes: 0,
                network_up_bytes: 0,
                temperature_celsius: temp,
                battery_percent: battery,
                uptime_seconds: uptime_secs,
                total_processes: 200,
                running_processes: 5,
            },
            processes: vec![],
            foreground: ForegroundInfo { name: "test".into(), title: "".into(), pid: 1234 },
            drives: vec![],
            gpu: GpuInfo { name: "Test GPU".into(), memory_mb: 4096, driver_version: "1.0".into() },
            interfaces: vec![],
            sys_info: SystemInfo {
                os_name: "TestOS".into(), os_version: "1.0".into(),
                kernel_version: "1.0".into(), hostname: "test".into(),
                cpu_brand: "Test CPU".into(), cpu_cores: 8, total_ram_mb: 16384,
            },
        }
    }

    #[test]
    fn test_severity_normal() {
        let snap = make_snap(20.0, 40, 40, 3600, 80, 40.0);
        let mut engine = AnalyticsEngine::new();
        let analysis = engine.analyze(&snap);
        assert!(analysis.system_health.severity_score <= 20,
            "normal load should give low severity, got {}", analysis.system_health.severity_score);
    }

    #[test]
    fn test_severity_high_memory() {
        let snap = make_snap(20.0, 95, 40, 3600, 80, 40.0);
        let mut engine = AnalyticsEngine::new();
        let analysis = engine.analyze(&snap);
        assert!(analysis.system_health.severity_score >= 30,
            "95% memory should give >=30 severity, got {}", analysis.system_health.severity_score);
    }

    #[test]
    fn test_severity_high_cpu() {
        let snap = make_snap(95.0, 40, 40, 3600, 80, 40.0);
        let mut engine = AnalyticsEngine::new();
        let analysis = engine.analyze(&snap);
        assert!(analysis.system_health.severity_score >= 20,
            "95% CPU should give >=20 severity, got {}", analysis.system_health.severity_score);
    }

    #[test]
    fn test_severity_long_uptime() {
        let snap = make_snap(20.0, 40, 40, 15 * 86400, 80, 40.0);
        let mut engine = AnalyticsEngine::new();
        let analysis = engine.analyze(&snap);
        assert!(analysis.system_health.severity_score >= 10,
            "15-day uptime should give >=10 severity, got {}", analysis.system_health.severity_score);
    }

    #[test]
    fn test_severity_low_battery() {
        let snap = make_snap(20.0, 40, 40, 3600, 5, 40.0);
        let mut engine = AnalyticsEngine::new();
        let analysis = engine.analyze(&snap);
        assert!(analysis.system_health.severity_score >= 10,
            "5% battery should give >=10 severity, got {}", analysis.system_health.severity_score);
    }

    #[test]
    fn test_anomaly_memory_saturation() {
        let snap = make_snap(20.0, 92, 40, 3600, 80, 40.0);
        let mut engine = AnalyticsEngine::new();
        let analysis = engine.analyze(&snap);
        assert!(analysis.anomalies.iter().any(|a| a.anomaly_type == "Memory Saturation"));
    }

    #[test]
    fn test_anomaly_memory_pressure() {
        let snap = make_snap(20.0, 85, 40, 3600, 80, 40.0);
        let mut engine = AnalyticsEngine::new();
        let analysis = engine.analyze(&snap);
        assert!(analysis.anomalies.iter().any(|a| a.anomaly_type == "Memory Pressure"));
    }

    #[test]
    fn test_anomaly_cpu_saturation() {
        let snap = make_snap(95.0, 40, 40, 3600, 80, 40.0);
        let mut engine = AnalyticsEngine::new();
        let analysis = engine.analyze(&snap);
        assert!(analysis.anomalies.iter().any(|a| a.anomaly_type == "CPU Saturation"));
    }

    #[test]
    fn test_anomaly_extended_uptime() {
        let snap = make_snap(20.0, 40, 40, 15 * 86400, 80, 40.0);
        let mut engine = AnalyticsEngine::new();
        let analysis = engine.analyze(&snap);
        assert!(analysis.anomalies.iter().any(|a| a.anomaly_type == "Extended Uptime"));
    }

    #[test]
    fn test_anomaly_memory_delta() {
        let mut engine = AnalyticsEngine::new();
        let snap1 = make_snap(20.0, 50, 40, 3600, 80, 40.0);
        let snap2 = make_snap(20.0, 60, 40, 3600, 80, 40.0);
        engine.add_snapshot(snap1);
        let analysis = engine.analyze(&snap2);
        assert!(analysis.anomalies.iter().any(|a| a.anomaly_type == "Memory Increase"));
    }

    #[test]
    fn test_trend_stable() {
        let mut engine = AnalyticsEngine::new();
        let snap1 = make_snap(30.0, 50, 40, 3600, 80, 40.0);
        let snap2 = make_snap(31.0, 51, 40, 3600, 80, 40.0);
        engine.add_snapshot(snap1);
        let analysis = engine.analyze(&snap2);
        assert_eq!(analysis.trend_analysis.cpu_trend, "Stable");
    }

    #[test]
    fn test_trend_increasing() {
        let mut engine = AnalyticsEngine::new();
        let snap1 = make_snap(20.0, 50, 40, 3600, 80, 40.0);
        let snap2 = make_snap(50.0, 50, 40, 3600, 80, 40.0);
        engine.add_snapshot(snap1);
        let analysis = engine.analyze(&snap2);
        assert_eq!(analysis.trend_analysis.cpu_trend, "Increasing sharply");
    }

    #[test]
    fn test_alerts_generated() {
        let snap = make_snap(85.0, 88, 92, 10 * 86400, 15, 65.0);
        let mut engine = AnalyticsEngine::new();
        let analysis = engine.analyze(&snap);
        assert!(!analysis.alerts.is_empty());
    }

    #[test]
    fn test_full_analysis_run() {
        let snap = make_snap(85.0, 88, 92, 10 * 86400, 15, 65.0);
        let mut engine = AnalyticsEngine::new();
        let analysis = engine.analyze(&snap);
        assert!(!analysis.anomalies.is_empty());
        assert!(analysis.system_health.severity_score > 30);
        assert!(!analysis.recommendations.is_empty());
        assert!(!analysis.observations.is_empty());
        assert!(!analysis.summary.is_empty());
        assert!(!analysis.performance.is_empty());
    }

    #[test]
    fn test_bottleneck_memory() {
        let snap = make_snap(30.0, 90, 40, 3600, 80, 40.0);
        let mut engine = AnalyticsEngine::new();
        let analysis = engine.analyze(&snap);
        assert!(analysis.resource_analysis.bottleneck.contains("Memory"));
    }

    #[test]
    fn test_bottleneck_cpu() {
        let snap = make_snap(90.0, 50, 40, 3600, 80, 40.0);
        let mut engine = AnalyticsEngine::new();
        let analysis = engine.analyze(&snap);
        assert!(analysis.resource_analysis.bottleneck.contains("CPU"));
    }

    #[test]
    fn test_focus_analysis() {
        let snap = make_snap(20.0, 40, 40, 3600, 80, 40.0);
        let mut engine = AnalyticsEngine::new();
        let analysis = engine.analyze(&snap);
        assert_eq!(analysis.focus_analysis.primary_app, "test");
    }

    #[test]
    fn test_summary_format() {
        let snap = make_snap(45.0, 60, 50, 86400, 80, 40.0);
        let mut engine = AnalyticsEngine::new();
        let analysis = engine.analyze(&snap);
        assert!(analysis.summary.contains("Severity score"));
        assert!(analysis.summary.contains("CPU"));
        assert!(analysis.summary.contains("memory"));
    }

    #[test]
    fn test_history_cap() {
        let mut engine = AnalyticsEngine::new();
        for _ in 0..150 {
            engine.add_snapshot(make_snap(20.0, 40, 40, 3600, 80, 40.0));
        }
        assert!(engine.history.len() <= 100);
    }
}
