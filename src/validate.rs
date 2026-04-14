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

    if let Some(limit) = budget.max_ram_overflow_bytes
        && report.linker.ram_overflow_bytes > limit
    {
        errors.push(format!(
            "RAM overflow budget exceeded: actual={} limit={limit}",
            report.linker.ram_overflow_bytes
        ));
    }

    if let Some(limit) = budget.max_uninit_overflow_bytes
        && report.linker.uninit_overflow_bytes > limit
    {
        errors.push(format!(
            "uninit overflow budget exceeded: actual={} limit={limit}",
            report.linker.uninit_overflow_bytes
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

    for (prefix, limit) in &budget.max_symbol_prefix_bytes {
        match report
            .top_symbols
            .iter()
            .find(|symbol| symbol.symbol.starts_with(prefix))
        {
            Some(symbol) if symbol.size_bytes > *limit => errors.push(format!(
                "symbol budget exceeded for [{prefix}]: actual={} limit={limit}",
                symbol.size_bytes
            )),
            Some(_) => {}
            None => errors.push(format!("symbol prefix not found in report: [{prefix}]")),
        }
    }

    errors
}
