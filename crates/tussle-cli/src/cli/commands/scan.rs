//! `tussle scan` — list every binding every source can see.

use anyhow::Result;
use tabled::builder::Builder;
use tabled::settings::Style;
use tussle_core::Binding;

use crate::cli::output::emit_json;
use crate::cli::sources::{default_sources, warn_if_no_accessibility};

pub fn scan(as_json: bool, ax_timeout: f32) -> Result<()> {
    let sources = default_sources(ax_timeout)?;
    warn_if_no_accessibility();

    let mut bindings: Vec<Binding> = Vec::new();
    for src in &sources {
        match src.scan() {
            Ok(found) => bindings.extend(found),
            Err(e) => eprintln!("{}: {:#}", src.name(), e),
        }
    }

    if as_json {
        return emit_json(&bindings);
    }

    if bindings.is_empty() {
        println!("(no bindings found)");
        return Ok(());
    }

    let mut builder = Builder::default();
    builder.push_record(["Combo", "Owner", "Action"]);
    for b in &bindings {
        builder.push_record([&format!("{}", b.combo), b.source.owner(), &b.label]);
    }
    println!("{}", builder.build().with(Style::psql()));
    Ok(())
}
