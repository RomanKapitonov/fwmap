use crate::{budget::FirmwareMemoryBudget, report::FirmwareMemoryReport};

pub fn validate_firmware_budget(
    report: &FirmwareMemoryReport,
    budget: &FirmwareMemoryBudget,
) -> Vec<String> {
    let mut errors = Vec::new();

    if budget.require_successful_build && !report.build.succeeded {
        errors.push(format!(
            "firmware build failed with exit code {}",
            report.build.exit_code
        ));
    }

    if report.linker.ram_overflow_bytes > budget.max_ram_overflow_bytes {
        errors.push(format!(
            "RAM overflow budget exceeded: actual={} limit={}",
            report.linker.ram_overflow_bytes, budget.max_ram_overflow_bytes
        ));
    }

    if report.linker.uninit_overflow_bytes > budget.max_uninit_overflow_bytes {
        errors.push(format!(
            "uninit overflow budget exceeded: actual={} limit={}",
            report.linker.uninit_overflow_bytes, budget.max_uninit_overflow_bytes
        ));
    }

    if budget.require_valid_stack_placement && report.linker.stack_placement_failed {
        errors.push("linker reported invalid stack placement".to_owned());
    }

    for (section, limit) in &budget.max_section_bytes {
        match report.sections.get(section) {
            Some(actual) if actual > limit => errors.push(format!(
                "section budget exceeded for [{section}]: actual={actual} limit={limit}"
            )),
            Some(_) => {}
            None => errors.push(format!("missing section [{section}] in map report")),
        }
    }

    // max_symbol_prefix_bytes: size-only constraint. If the prefix
    // does not match any top-N symbol, silently skip (the symbol may
    // have been inlined or DCE-stripped).
    for (prefix, limit) in &budget.max_symbol_prefix_bytes {
        if let Some(symbol) = report
            .top_symbols
            .iter()
            .find(|symbol| symbol.symbol.starts_with(prefix))
            && symbol.size_bytes > *limit
        {
            errors.push(format!(
                "symbol budget exceeded for [{prefix}]: actual={} limit={limit}",
                symbol.size_bytes
            ));
        }
    }

    // required_symbols: must exist AND fit under limit. Failure if
    // prefix not found, failure if found AND oversized.
    for (prefix, limit) in &budget.required_symbols {
        match report
            .top_symbols
            .iter()
            .find(|symbol| symbol.symbol.starts_with(prefix))
        {
            Some(symbol) if symbol.size_bytes > *limit => errors.push(format!(
                "required symbol budget exceeded for [{prefix}]: actual={} limit={limit}",
                symbol.size_bytes
            )),
            Some(_) => {}
            None => errors.push(format!(
                "required symbol prefix not found in report: [{prefix}]"
            )),
        }
    }

    errors
}
