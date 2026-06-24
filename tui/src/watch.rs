//! `kontu watch` plumbing: a persisted set of already-notified listing ids (so a
//! scheduled run only alerts on genuinely new matches), Telegram alert formatting,
//! and a systemd-user timer installer (the residential machine must do the polling
//! because the Worker's datacenter IP is portal-blocked).

use std::collections::BTreeSet;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::config::Config;
use crate::matching::Scored;
use crate::telegram::escape;

/// Path of the notified-ids file, next to `config.toml` in the kontu config dir.
fn seen_path() -> Result<PathBuf> {
    let dir = Config::config_path()?
        .parent()
        .context("config dir has no parent")?
        .to_path_buf();
    Ok(dir.join("seen.json"))
}

/// Load the set of listing ids already pushed to Telegram (empty on first run).
/// A corrupt file is a hard error, not an empty set — silently resetting the
/// baseline would re-alert every current match at once.
pub fn load_seen() -> Result<BTreeSet<i64>> {
    let path = seen_path()?;
    if !path.exists() {
        return Ok(BTreeSet::new());
    }
    let text = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| {
        format!("{} is corrupt — delete it to re-baseline (this re-alerts current matches)", path.display())
    })
}

/// Persist the notified-ids set with an atomic temp-file + rename so a crash or a
/// concurrent run can never leave a truncated (corrupt) baseline behind.
pub fn save_seen(seen: &BTreeSet<i64>) -> Result<()> {
    let path = seen_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    let text = serde_json::to_string(seen).context("serializing seen set")?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text).with_context(|| format!("writing {}", tmp.display()))?;
    std::fs::rename(&tmp, &path).with_context(|| format!("replacing {}", path.display()))
}

/// Render a ranked match as a Telegram HTML alert. The bare URL on its own line
/// makes Telegram render the listing's link preview (cover photo) inline.
pub fn format_alert(m: &Scored) -> String {
    let price = m
        .price_eur
        .map(|p| format!("{} €", thousands(p)))
        .unwrap_or_else(|| "price on request".to_string());
    let place = m.municipality.as_deref().unwrap_or("?");
    let reasons = if m.reasons.is_empty() {
        String::new()
    } else {
        format!("\n<i>{}</i>", escape(&m.reasons.join(", ")))
    };
    format!(
        "🏠 <b>{title}</b>\n📍 {place}\n💶 {price} · ~{monthly} €/mo · risk {risk} · fit {fit:.0}{reasons}\n{url}",
        title = escape(&m.title),
        place = escape(place),
        monthly = m.monthly.round() as i64,
        risk = m.risk,
        fit = m.score,
        url = escape(&m.url),
    )
}

fn thousands(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(c);
    }
    if n < 0 {
        format!("-{out}")
    } else {
        out
    }
}

/// Write a systemd-user service+timer that runs `kontu watch run` on a schedule
/// and reload the daemon. Enables the timer only when `enable` is set (i.e. when
/// Telegram is configured) so we never leave a timer that fails every cycle.
/// Returns a human summary of what was installed.
pub fn install_timer(schedule: Option<String>, enable: bool) -> Result<String> {
    let oncalendar = schedule.unwrap_or_else(|| "*-*-* 08,10,12,14,16,18,20,22:00:00".to_string());
    let exe = std::env::current_exe()
        .context("locating the kontu executable")?
        .display()
        .to_string();
    let base = directories::BaseDirs::new().context("could not determine the home directory")?;
    let unit_dir = base.config_dir().join("systemd/user");
    std::fs::create_dir_all(&unit_dir)
        .with_context(|| format!("creating {}", unit_dir.display()))?;

    let service = format!(
        "[Unit]\n\
         Description=kontu — one new-listing detection cycle (pull + match + Telegram alerts)\n\
         After=network-online.target\n\
         Wants=network-online.target\n\n\
         [Service]\n\
         Type=oneshot\n\
         ExecStart={exe} watch run\n"
    );
    let timer = format!(
        "[Unit]\n\
         Description=kontu watch schedule\n\n\
         [Timer]\n\
         OnCalendar={oncalendar}\n\
         Persistent=true\n\
         RandomizedDelaySec=300\n\n\
         [Install]\n\
         WantedBy=timers.target\n"
    );
    let service_path = unit_dir.join("kontu-watch.service");
    let timer_path = unit_dir.join("kontu-watch.timer");
    std::fs::write(&service_path, service)
        .with_context(|| format!("writing {}", service_path.display()))?;
    std::fs::write(&timer_path, timer)
        .with_context(|| format!("writing {}", timer_path.display()))?;

    let reloaded = run_systemctl(&["daemon-reload"]).is_ok();

    let mut out = format!(
        "installed:\n  {}\n  {}\nschedule: {oncalendar}",
        service_path.display(),
        timer_path.display()
    );
    if !enable {
        out.push_str(
            "\nnot enabled (Telegram not configured) — set it up, then run:\n  systemctl --user enable --now kontu-watch.timer",
        );
    } else if reloaded && run_systemctl(&["enable", "--now", "kontu-watch.timer"]).is_ok() {
        out.push_str("\nenabled: kontu-watch.timer (systemctl --user)");
    } else {
        out.push_str(
            "\ncould not auto-enable — run:\n  systemctl --user daemon-reload\n  systemctl --user enable --now kontu-watch.timer",
        );
    }
    Ok(out)
}

fn run_systemctl(args: &[&str]) -> Result<()> {
    let status = std::process::Command::new("systemctl")
        .arg("--user")
        .args(args)
        .status()
        .context("running systemctl --user")?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("systemctl --user {:?} exited with {status}", args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thousands_groups_by_three() {
        assert_eq!(thousands(89_000), "89 000");
        assert_eq!(thousands(1_250_000), "1 250 000");
        assert_eq!(thousands(950), "950");
    }

    #[test]
    fn alert_escapes_url_and_title_for_telegram_html() {
        let m = Scored {
            id: 1,
            title: "Talo & <tontti>".into(),
            municipality: Some("Outokumpu".into()),
            price_eur: Some(89_000),
            property_type: Some("omakotitalo".into()),
            url: "https://x.fi/k?a=1&b=2".into(),
            score: 80.0,
            npv_cost: 0.0,
            monthly: 700.0,
            risk: 10,
            reasons: vec!["lakeshore".into()],
        };
        let out = format_alert(&m);
        assert!(out.contains("Talo &amp; &lt;tontti&gt;"), "title not escaped: {out}");
        assert!(out.contains("a=1&amp;b=2"), "url '&' not escaped (would break Telegram HTML parse): {out}");
        assert!(!out.contains("a=1&b=2"), "raw unescaped '&' must not appear: {out}");
    }
}
