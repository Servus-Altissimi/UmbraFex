// escape HTML special characters so tokenised text is safe 
#[inline]
pub fn escape_html_char(c: char, out: &mut String) {
    match c {
        '&'  => out.push_str("&amp;"),
        '<'  => out.push_str("&lt;"),
        '>'  => out.push_str("&gt;"),
        '"'  => out.push_str("&quot;"),
        '\'' => out.push_str("&#39;"),
        _    => out.push(c),
    }
}

// Classify an identifier as: keyword, type, builtin function, or plain ident
// Sourced from Sonnet 4.6, so I'm not certain it's complete
pub fn classify_word(w: &str) -> &'static str {
    match w {
        // control flow and declaration keywords
        "fn" | "var" | "let" | "const" | "struct" | "return" |
        "if" | "else" | "for" | "while" | "loop" | "break" |
        "continue" | "switch" | "case" | "default" | "true" | "false" |
        "override" | "discard" | "continuing" | "enable" | "requires" |
        "diagnostic" |

        // storage & address-space qualifiers
        "storage" | "workgroup" | "private" | "function" | "read" |
        "write" | "read_write" | "uniform" | "push_constant" => "hl-kw",

        // scalar and aggregate types
        "f32" | "f16" | "i32" | "u32" | "bool" |
        "vec2"  | "vec3"  | "vec4"  |
        "vec2f" | "vec3f" | "vec4f" |
        "vec2i" | "vec3i" | "vec4i" |
        "vec2u" | "vec3u" | "vec4u" |
        "vec2h" | "vec3h" | "vec4h" |
        "mat2x2" | "mat2x3" | "mat2x4" |
        "mat3x2" | "mat3x3" | "mat3x4" |
        "mat4x2" | "mat4x3" | "mat4x4" |
        "mat2x2f" | "mat3x3f" | "mat4x4f" |
        "array" | "atomic" | "ptr" |
        "texture_1d" | "texture_2d" | "texture_2d_array" |
        "texture_3d" | "texture_cube" | "texture_cube_array" |
        "texture_multisampled_2d" | "texture_depth_2d" |
        "texture_depth_cube" | "texture_depth_2d_array" |
        "texture_depth_cube_array" | "texture_depth_multisampled_2d" |
        "texture_storage_1d" | "texture_storage_2d" | "texture_storage_2d_array" |
        "texture_storage_3d" | "sampler" | "sampler_comparison" => "hl-type",

        // built-in functions
        "abs" | "acos" | "acosh" | "asin" | "asinh" | "atan" | "atanh" | "atan2" |
        "ceil" | "clamp" | "cos" | "cosh" | "cross" | "degrees" | "determinant" |
        "distance" | "dot" | "exp" | "exp2" | "faceForward" | "floor" | "fma" |
        "fract" | "frexp" | "inverseSqrt" | "ldexp" | "length" | "log" | "log2" |
        "max" | "min" | "mix" | "modf" | "normalize" | "pow" | "quantizeToF16" |
        "radians" | "reflect" | "refract" | "reverseBits" | "round" | "saturate" |
        "sign" | "sin" | "sinh" | "smoothstep" | "sqrt" | "step" | "tan" | "tanh" |
        "transpose" | "trunc" | "select" | "all" | "any" | "arrayLength" |
        "bitcast" | "countLeadingZeros" | "countOneBits" | "countTrailingZeros" |
        "extractBits" | "firstLeadingBit" | "firstTrailingBit" | "insertBits" |
        "dpdx" | "dpdxCoarse" | "dpdxFine" | "dpdy" | "dpdyCoarse" | "dpdyFine" |
        "fwidth" | "fwidthCoarse" | "fwidthFine" |
        "textureDimensions" | "textureGather" | "textureGatherCompare" |
        "textureLoad" | "textureNumLayers" | "textureNumLevels" |
        "textureNumSamples" | "textureSample" | "textureSampleBias" |
        "textureSampleCompare" | "textureSampleCompareLevel" |
        "textureSampleGrad" | "textureSampleLevel" | "textureStore" |
        "workgroupBarrier" | "storageBarrier" | "textureBarrier" |
        "atomicAdd" | "atomicAnd" | "atomicExchange" | "atomicLoad" | "atomicMax" |
        "atomicMin" | "atomicOr" | "atomicStore" | "atomicSub" | "atomicXor" |
        "pack2x16float" | "pack2x16snorm" | "pack2x16unorm" |
        "pack4x8snorm" | "pack4x8unorm" |
        "unpack2x16float" | "unpack2x16snorm" | "unpack2x16unorm" |
        "unpack4x8snorm" | "unpack4x8unorm" => "hl-builtin",

        _ => "hl-ident",
    }
}

pub fn parse_err_lines(err: &str) -> Vec<usize> {
    err.lines()
        .filter_map(|l| l.strip_prefix("line ")?.split(':').next()?.trim().parse().ok())
        .collect()
}

// Single-pass WGSL tokeniser; returns an HTML string safe for dangerous_inner_html.
// It's a little amateur, but it does the job
// Recognised token classes:
//   hl-comment — // line and /* block */ comments
//   hl-attr    — @attribute decorators
//   hl-num     — numeric literals (decimal, hex, float, suffixed)
//   hl-kw      — language keywords and storage qualifiers
//   hl-type    — scalar, vector, matrix and texture types
//   hl-builtin — standard library functions
//   hl-ident   — everything else (user identifiers)
pub fn highlight_wgsl(src: &str, err_lines: &[usize]) -> String {
    let chars: Vec<char> = src.chars().collect();
    let len   = chars.len();
    let mut out   = String::with_capacity(src.len() * 3);
    let mut i     = 0usize;
    
    // Separate depth counters for {} and () colouring
    let mut depth_brace = 0usize;
    let mut depth_paren = 0usize;

    while i < len {
        // line comments
        if i + 1 < len && chars[i] == '/' && chars[i + 1] == '/' {
            let start = i;
            while i < len && chars[i] != '\n' { i += 1; }
            out.push_str("<span class='hl-comment'>");
            for c in &chars[start..i] { escape_html_char(*c, &mut out); }
            out.push_str("</span>");
            continue;
        }

        // block comments
        if i + 1 < len && chars[i] == '/' && chars[i + 1] == '*' {
            let start = i;
            i += 2;
            while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') { i += 1; }
            if i + 1 < len { i += 2; } // consume closing */
            out.push_str("<span class='hl-comment'>");
            for c in &chars[start..i] { escape_html_char(*c, &mut out); }
            out.push_str("</span>");
            continue;
        }

        // attribute (@ident)
        if chars[i] == '@' {
            let start = i;
            i += 1;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') { i += 1; }
            out.push_str("<span class='hl-attr'>");
            for c in &chars[start..i] { escape_html_char(*c, &mut out); }
            out.push_str("</span>");
            continue;
        }

        // number literals
        // also catches hex and type-suffixed literals
        if chars[i].is_ascii_digit()
            || (chars[i] == '.' && i + 1 < len && chars[i + 1].is_ascii_digit())
        {
            let start = i;
            while i < len
                && (chars[i].is_ascii_alphanumeric() || chars[i] == '.' || chars[i] == '_')
            { i += 1; }
            out.push_str("<span class='hl-num'>");
            for c in &chars[start..i] { escape_html_char(*c, &mut out); }
            out.push_str("</span>");
            continue;
        }

        // identifier & keyword & type & builtin
        if chars[i].is_alphabetic() || chars[i] == '_' {
            let start = i;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') { i += 1; }
            let tok: String = chars[start..i].iter().collect();
            let cls = classify_word(&tok);
            out.push_str("<span class='");
            out.push_str(cls);
            out.push_str("'>");
            // identifiers are ASCII-safe, but escape anyway
            for c in &chars[start..i] { escape_html_char(*c, &mut out); }
            out.push_str("</span>");
            continue;
        }

        // curly braces
        // Opening: use current depth for colour, then increment.
        // Closing: decrement first so the closing brace shares its opener's colour.
        if chars[i] == '{' {
            let cls = depth_brace % 3;
            depth_brace += 1;
            out.push_str("<span class='hl-brace-"); out.push_str(&cls.to_string());
            out.push_str("'>{</span>");
            i += 1;
            continue;
        }
        if chars[i] == '}' {
            if depth_brace > 0 { depth_brace -= 1; }
            let cls = depth_brace % 3;
            out.push_str("<span class='hl-brace-"); out.push_str(&cls.to_string());
            out.push_str("'>}</span>");
            i += 1;
            continue;
        }

        // parentheses, same logic as curly braces
        if chars[i] == '(' {
            let cls = depth_paren % 3;
            depth_paren += 1;
            out.push_str("<span class='hl-paren-"); out.push_str(&cls.to_string());
            out.push_str("'>(</span>");
            i += 1;
            continue;
        }
        if chars[i] == ')' {
            if depth_paren > 0 { depth_paren -= 1; }
            let cls = depth_paren % 3;
            out.push_str("<span class='hl-paren-"); out.push_str(&cls.to_string());
            out.push_str("'>)</span>");
            i += 1;
            continue;
        }

        // operators and punctuation
        if matches!(chars[i],
            '+' | '-' | '*' | '/' | '%' | '=' | '!' | '<' | '>' |
            '&' | '|' | '^' | '~' | '?' | ':' | ';' | ',' | '.' |
            '[' | ']'
        ) {
            out.push_str("<span class='hl-op'>");
            escape_html_char(chars[i], &mut out);
            out.push_str("</span>");
            i += 1;
            continue;
        }
        // everything else (whitespace, newlines)
        escape_html_char(chars[i], &mut out);
        i += 1;
    }

    //post-process: split by line and wrap error lines
    if err_lines.is_empty() { return out; }
    out.split('\n')
        .enumerate()
        .map(|(i, line)| {
            if err_lines.contains(&(i + 1)) {
                format!("<span class='hl-err-line'>{line}</span>")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
