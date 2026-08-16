# .github/workflows/modules/config.nu

export const RELEASE_DIR = "target/release"
export const OUTPUT_DIR = "output"

export def get-version [config?: record] {
    if $config != null {
        try { $config.metadata.version } catch { "" }
    } else if ("Cargo.toml" | path exists) {
        open Cargo.toml | get package.version
    } else {
        ""
    }
}

export def hr-line [--blank_line(-b)] {
    print $"(ansi g)---------------------------------------------------------------------------->(ansi reset)"
    if $blank_line { char nl }
}

export def format_template [
    template_path: string
    context: record
] {
    let eval_condition = {|cond: string|
        if ($cond | str starts-with "!") {
            let c = ($cond | str substring 1..)
            let val = (try { $context | get $c } catch { false })
            not $val
        } else {
            (try { $context | get $cond } catch { false })
        }
    }

    mut filtered_lines = []
    mut skip_stack = []

    for line in (open --raw $template_path | lines) {
        let start_match = ($line | parse -r '^\s*\[IF\s+(?<condition>[a-zA-Z0-9._!-]+)\]\s*$')
        if ($start_match | is-not-empty) {
            let cond = $start_match.0.condition
            let is_cond_true = (do $eval_condition $cond)
            let parent_skip = if ($skip_stack | is-empty) { false } else { $skip_stack | last }
            $skip_stack = ($skip_stack | append ($parent_skip or not $is_cond_true))
            continue
        }
        
        let end_match = ($line | parse -r '^\s*\[/IF\]\s*$')
        if ($end_match | is-not-empty) {
            if not ($skip_stack | is-empty) {
                $skip_stack = ($skip_stack | drop 1)
            }
            continue
        }
        
        let skip_line = if ($skip_stack | is-empty) { false } else { $skip_stack | last }
        
        if not $skip_line {
            $filtered_lines = ($filtered_lines | append $line)
        }
    }

    $filtered_lines | str join (char nl)
}

export def load-config [config_file: string = "release.toml"] {
    if not ($config_file | path exists) {
        print $"::error::($config_file) not found."
        exit 1
    }
    open $config_file
}
