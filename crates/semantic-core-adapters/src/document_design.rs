use crate::{
    knowledge_work::{
        BusinessDocumentIR, BusinessDocumentTypeIR, ChartIR, DocumentDesignIR, DocumentThemeIR,
        FinancialStatementIR, KnowledgeDocumentIR, KnowledgeFindingIR, PageSizeIR, PaperIR,
        PlanProposalIR, TableIR, UserGuideIR,
    },
    language_knowledge::LanguageCodeIR,
};

pub(crate) fn render_print_ready_html(
    document: &KnowledgeDocumentIR,
    findings: &[KnowledgeFindingIR],
    language: LanguageCodeIR,
    design: &DocumentDesignIR,
) -> String {
    let korean = language == LanguageCodeIR::Korean;
    let tokens = DesignTokens::resolve(design);
    let title = document_title(document);
    let body = match document {
        KnowledgeDocumentIR::Paper(paper) => render_paper(paper, findings, korean, design),
        KnowledgeDocumentIR::BusinessPlan(business)
        | KnowledgeDocumentIR::BusinessProposal(business) => {
            render_business(business, findings, korean, design)
        }
        KnowledgeDocumentIR::UserGuide(guide) => render_user_guide(guide, findings, korean, design),
        KnowledgeDocumentIR::Table(table) => render_table_document(table, findings, korean),
        KnowledgeDocumentIR::Chart(chart) => render_chart_document(chart, findings, korean),
        KnowledgeDocumentIR::FinancialStatement(statement) => {
            render_financial_document(statement, findings, korean)
        }
        KnowledgeDocumentIR::PlanProposal(plan) => render_plan_document(plan, findings, korean),
    };
    format!(
        "<!doctype html><html lang=\"{}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{}</title><style>{}</style></head><body class=\"{}\"><main class=\"document-shell\">{}</main></body></html>",
        if korean { "ko" } else { "en" },
        html_escape(title),
        stylesheet(&tokens, design),
        tokens.theme_class,
        body,
    )
}

struct DesignTokens {
    theme_class: &'static str,
    ink: &'static str,
    muted: &'static str,
    paper: &'static str,
    surface: &'static str,
    accent: String,
    accent_soft: &'static str,
    line: &'static str,
    display_font: &'static str,
    body_font: &'static str,
}

impl DesignTokens {
    fn resolve(design: &DocumentDesignIR) -> Self {
        let mut tokens = match design.theme {
            DocumentThemeIR::AcademicEditorial => Self {
                theme_class: "theme-academic",
                ink: "#182126",
                muted: "#59656b",
                paper: "#fbfaf6",
                surface: "#f1efe8",
                accent: "#a33b2b".to_string(),
                accent_soft: "#efe2dd",
                line: "#d6d1c5",
                display_font: "'Iowan Old Style','Palatino Linotype','Noto Serif KR',serif",
                body_font: "'Noto Serif KR','Book Antiqua',Georgia,serif",
            },
            DocumentThemeIR::ExecutiveNavy => Self {
                theme_class: "theme-executive",
                ink: "#10243a",
                muted: "#5d6d7e",
                paper: "#f6f8fa",
                surface: "#e9eef3",
                accent: "#16a085".to_string(),
                accent_soft: "#d9f0eb",
                line: "#ccd6df",
                display_font: "'Bahnschrift','Aptos Display','Noto Sans KR',sans-serif",
                body_font: "'Noto Sans KR','Segoe UI Variable',sans-serif",
            },
            DocumentThemeIR::ProposalCobalt => Self {
                theme_class: "theme-proposal",
                ink: "#13203d",
                muted: "#60708e",
                paper: "#f8f9ff",
                surface: "#edf1ff",
                accent: "#2457d6".to_string(),
                accent_soft: "#dfe8ff",
                line: "#cbd5ef",
                display_font: "'Aptos Display','Malgun Gothic','Noto Sans KR',sans-serif",
                body_font: "'Noto Sans KR','Segoe UI Variable',sans-serif",
            },
            DocumentThemeIR::GuideIndigo => Self {
                theme_class: "theme-guide",
                ink: "#18232d",
                muted: "#596873",
                paper: "#ffffff",
                surface: "#f4f6f7",
                accent: "#1f4e79".to_string(),
                accent_soft: "#e8eef3",
                line: "#cfd6dc",
                display_font: "'KoPubWorld Batang','Noto Serif KR','Batang',serif",
                body_font: "'KoPubWorld Dotum','Noto Sans KR','Malgun Gothic',sans-serif",
            },
            DocumentThemeIR::MinimalMonochrome => Self {
                theme_class: "theme-minimal",
                ink: "#17191b",
                muted: "#676d71",
                paper: "#ffffff",
                surface: "#f2f3f3",
                accent: "#17191b".to_string(),
                accent_soft: "#e8e9e9",
                line: "#d9dcde",
                display_font: "'Franklin Gothic Medium','Noto Sans KR',sans-serif",
                body_font: "'Noto Sans KR','Segoe UI Variable',sans-serif",
            },
        };
        if let Some(accent) = design.accent_color.as_ref() {
            tokens.accent.clone_from(accent);
        }
        tokens
    }
}

fn stylesheet(tokens: &DesignTokens, design: &DocumentDesignIR) -> String {
    let (page, page_width, page_height) = match design.page_size {
        PageSizeIR::A4 => ("A4", "210mm", "297mm"),
        PageSizeIR::Letter => ("Letter", "215.9mm", "279.4mm"),
    };
    let section_gap = if design.compact { "5mm" } else { "8mm" };
    let furniture_visibility = if design.show_page_furniture {
        "visible"
    } else {
        "hidden"
    };
    format!(
        r#"
@page {{ size: {page}; margin:0; }}
:root {{ --ink:{ink}; --muted:{muted}; --paper:{paper}; --surface:{surface}; --accent:{accent}; --accent-soft:{accent_soft}; --line:{line}; --display:{display}; --body:{body}; --section-gap:{section_gap}; --page-width:{page_width}; --page-height:{page_height}; --furniture:{furniture_visibility}; }}
* {{ box-sizing:border-box; }}
html {{ background:#d7dade;color:var(--ink);font-family:var(--body);font-size:10.5pt;line-height:1.62;text-rendering:optimizeLegibility; }}
body {{ margin:0;min-height:100vh;background:#d7dade; }}
.document-shell {{ width:100%;margin:0;padding:14mm 0 22mm;display:flex;flex-direction:column;align-items:center;gap:10mm;counter-reset:sheet; }}
.sheet {{ width:var(--page-width);min-height:var(--page-height);padding:21mm 18mm 18mm;background:var(--paper);border:1px solid #c5c9cc;box-shadow:0 4mm 12mm rgba(20,31,41,.16);position:relative;overflow:hidden;counter-increment:sheet;break-after:page;page-break-after:always; }}
.sheet::before {{ content:attr(data-running-title);visibility:var(--furniture);position:absolute;top:10mm;left:18mm;right:18mm;padding-bottom:3mm;border-bottom:.25mm solid var(--line);color:var(--muted);font:600 8pt/1.2 var(--body);letter-spacing:.03em; }}
.sheet::after {{ content:counter(sheet);visibility:var(--furniture);position:absolute;bottom:9mm;right:18mm;color:var(--muted);font:600 8pt/1 var(--body);font-variant-numeric:tabular-nums; }}
.cover {{ padding:24mm 20mm 20mm;display:flex;flex-direction:column;justify-content:space-between; }}
.cover::before {{ content:'';position:absolute;left:20mm;top:20mm;width:24mm;height:1.4mm;background:var(--accent); }}
.cover::after {{ content:none; }}
.cover.sheet::before {{ display:none; }}
.eyebrow {{ display:inline-flex;color:var(--accent);text-transform:uppercase;letter-spacing:.16em;font:700 8.5pt/1 var(--body); }}
.eyebrow::before {{ content:none; }}
.cover h1 {{ max-width:150mm;margin:19mm 0 8mm;font:700 30pt/1.18 var(--display);letter-spacing:-.025em;text-wrap:balance; }}
.cover-deck {{ max-width:145mm;color:var(--muted);font-size:11.5pt;line-height:1.72; }}
.cover-meta {{ display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:8mm;padding-top:8mm;border-top:.35mm solid var(--ink); }}
.meta-label {{ color:var(--muted);letter-spacing:.04em;font-size:7.5pt;font-weight:700; }}
.meta-value {{ margin-top:2mm;font:650 9.5pt/1.4 var(--body); }}
.content {{ padding:21mm 18mm 18mm; }}
.toc {{ margin:0 0 10mm;padding:7mm 8mm;border:.25mm solid var(--line);background:var(--paper);break-inside:avoid; }}
.toc-title {{ margin:0 0 4mm;color:var(--ink);font:700 9pt/1 var(--body);letter-spacing:.04em; }}
.toc-grid {{ display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:2mm 8mm;counter-reset:toc; }}
.toc a {{ color:var(--ink);text-decoration:none;border-bottom:.2mm dotted var(--line);padding:2mm 0;display:flex;gap:4mm; }}
.toc a::before {{ counter-increment:toc;content:counter(toc,decimal-leading-zero);color:var(--accent);font-weight:700; }}
.section {{ margin-top:var(--section-gap); break-inside:auto; }}
.section-kicker {{ color:var(--accent);font:700 7.5pt/1 var(--body);letter-spacing:.1em;text-transform:uppercase;margin-bottom:2.5mm; }}
.section h2 {{ margin:0 0 5mm;font:700 18pt/1.3 var(--display);letter-spacing:-.012em; }}
.section h3 {{ margin:7mm 0 3mm;font:700 13pt/1.35 var(--display); }}
.section-body {{ color:var(--ink);white-space:pre-wrap;max-width:76ch; }}
.lead {{ font-size:11.5pt;line-height:1.75;color:var(--ink);max-width:75ch; }}
.dropcap::first-letter {{ float:none;font:inherit;color:inherit;padding:0; }}
.metric-grid {{ display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:3mm;margin:6mm 0 8mm; }}
.metric {{ background:var(--surface);padding:5mm 4mm;border-top:.8mm solid var(--accent);break-inside:avoid; }}
.metric-label {{ color:var(--muted);font-size:11px;font-weight:700;letter-spacing:.08em;text-transform:uppercase; }}
.metric-value {{ margin:3mm 0 1mm;font:700 17pt/1 var(--display);letter-spacing:-.02em; }}
.metric-change {{ color:var(--accent);font-size:12px;font-weight:700; }}
.highlight-grid {{ display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:3mm;margin-top:5mm; }}
.highlight {{ padding:4mm 5mm;background:var(--surface);border-left:.8mm solid var(--accent);font-weight:600;break-inside:avoid; }}
.callout {{ margin:7mm 0;padding:5mm 6mm;background:var(--surface);color:var(--ink);border:.25mm solid var(--line);border-left:1.2mm solid var(--accent);break-inside:avoid; }}
.callout::after {{ content:none; }}
.callout-label {{ color:var(--accent);letter-spacing:.04em;font-size:8pt;font-weight:800; }}
.callout p {{ margin:2mm 0 0;font:600 10.5pt/1.6 var(--body);max-width:none; }}
.table-wrap {{ margin:6mm 0 8mm;overflow:hidden;border:.25mm solid var(--line);break-inside:avoid; }}
.table-title {{ padding:3.5mm 4mm;background:var(--surface);color:var(--ink);border-bottom:.25mm solid var(--line);font:700 10pt/1.2 var(--body); }}
table {{ width:100%;border-collapse:collapse;font-variant-numeric:tabular-nums; }}
th {{ padding:3mm 3.5mm;background:var(--accent-soft);color:var(--ink);text-align:left;font-size:8pt;border-bottom:.25mm solid var(--line); }}
td {{ padding:3mm 3.5mm;border-bottom:.2mm solid var(--line);vertical-align:top; }}
tr:nth-child(even) td {{ background:color-mix(in srgb,var(--surface) 55%,transparent); }}
td.numeric {{ text-align:right;font-weight:700; }}
.chart-card {{ margin:6mm 0 8mm;padding:4mm;background:var(--paper);border:.25mm solid var(--line);break-inside:avoid; }}
.chart-card svg {{ display:block;width:100%;height:auto;max-height:520px; }}
.timeline {{ display:grid;gap:0;margin:6mm 0; }}
.timeline-item {{ display:grid;grid-template-columns:94px 1fr;gap:22px;padding:0 0 28px;position:relative; }}
.timeline-item::before {{ content:'';position:absolute;left:45px;top:26px;bottom:-2px;width:1px;background:var(--line); }}
.timeline-item:last-child::before {{ display:none; }}
.timeline-id {{ width:90px;height:30px;display:grid;place-items:center;background:var(--accent-soft);color:var(--accent);border:.25mm solid var(--accent);font:800 10px/1 var(--body);letter-spacing:.07em;z-index:1; }}
.timeline-copy h4 {{ margin:1px 0 7px;font:700 17px/1.25 var(--display); }}
.timeline-copy p {{ margin:0;color:var(--muted);font-size:13px; }}
.guide-steps {{ counter-reset:guide-step;display:grid;gap:2mm;margin:5mm 0;padding-left:7mm; }}
.guide-steps li {{ counter-increment:guide-step;padding:2mm 2mm 2mm 1mm;border-bottom:.2mm solid var(--line);break-inside:avoid; }}
.guide-steps li::marker {{ color:var(--accent);font-weight:700; }}
.example-grid {{ display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:3mm;margin:5mm 0; }}
.example-card {{ padding:4mm;border:.25mm solid var(--line);background:var(--surface);break-inside:avoid; }}
.example-card h3 {{ margin:0 0 14px;font:700 18px/1.3 var(--display); }}
.example-label {{ margin:14px 0 5px;color:var(--accent);font-size:10px;font-weight:800;letter-spacing:.1em;text-transform:uppercase; }}
.example-copy {{ margin:0;color:var(--ink);font-size:13px; }}
.trouble-grid {{ display:grid;gap:1px;background:var(--line);border:1px solid var(--line); }}
.trouble-item {{ display:grid;grid-template-columns:minmax(150px,.75fr) 1.5fr;gap:22px;padding:18px 20px;background:var(--paper);break-inside:avoid; }}
.trouble-symptom {{ font-weight:750;color:var(--ink); }} .trouble-resolution {{ color:var(--muted); }}
.checklist {{ display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:2mm;padding:0;list-style:none; }}
.checklist li {{ position:relative;padding:3mm 3mm 3mm 10mm;background:var(--paper);border:.25mm solid var(--line);font-weight:650;break-inside:avoid; }}
.checklist li::before {{ content:'';position:absolute;left:3.5mm;top:3.5mm;width:4mm;height:4mm;border:.35mm solid var(--accent); }}
.finding-list {{ display:grid;gap:10px;margin-top:18px; }}
.finding {{ display:grid;grid-template-columns:110px 1fr;gap:18px;padding:15px 0;border-bottom:1px solid var(--line); }}
.finding-kind {{ color:var(--accent);font-size:10px;font-weight:800;letter-spacing:.08em;text-transform:uppercase; }}
.finding-copy {{ font-size:13px; }}
.reference-list {{ padding-left:20px;color:var(--muted);font-size:13px; }}
.page-break {{ break-before:page; }}
.cta {{ margin-top:8mm;padding:6mm;background:var(--surface);color:var(--ink);border:.25mm solid var(--line);border-top:1mm solid var(--accent);display:grid;grid-template-columns:1fr auto;gap:8mm;align-items:center;break-inside:avoid; }}
.cta h3 {{ margin:0 0 2mm;font:700 15pt/1.3 var(--display); }} .cta p {{ margin:0;color:var(--muted);max-width:62ch; }}
.cta-mark {{ display:none; }}
.theme-academic .cover::before {{ background:var(--accent); }}
.theme-academic .cover h1 {{ font-weight:600; }}
.theme-proposal .cover::before,.theme-guide .cover::before {{ background:var(--accent); }}
@media (max-width:900px) {{ .document-shell {{ padding:0;gap:8px;align-items:stretch; }} .sheet {{ width:100%;min-height:auto;padding:56px 34px 64px;box-shadow:none;border-left:0;border-right:0; }} .cover {{ min-height:100vh; }} .cover-meta,.metric-grid,.highlight-grid,.toc-grid,.example-grid,.checklist {{ grid-template-columns:1fr 1fr; }} .trouble-item {{ grid-template-columns:1fr;gap:6px; }} }}
@media print {{ html,body {{ background:white; }} .document-shell {{ display:block;padding:0; }} .sheet {{ width:var(--page-width);min-height:var(--page-height);margin:0;box-shadow:none;border:0; }} .chart-card,.table-wrap,.metric,.callout,.cta {{ break-inside:avoid; }} a {{ color:inherit; }} }}
"#,
        page = page,
        page_width = page_width,
        page_height = page_height,
        ink = tokens.ink,
        muted = tokens.muted,
        paper = tokens.paper,
        surface = tokens.surface,
        accent = tokens.accent,
        accent_soft = tokens.accent_soft,
        line = tokens.line,
        display = tokens.display_font,
        body = tokens.body_font,
        section_gap = section_gap,
        furniture_visibility = furniture_visibility,
    )
}

fn render_paper(
    paper: &PaperIR,
    findings: &[KnowledgeFindingIR],
    korean: bool,
    design: &DocumentDesignIR,
) -> String {
    let authors = if paper.authors.is_empty() {
        if korean {
            "미지정".to_string()
        } else {
            "Not specified".to_string()
        }
    } else {
        paper.authors.join(", ")
    };
    let section_count = paper.sections.len().to_string();
    let claim_count = paper.claims.len().to_string();
    let sections = paper
        .sections
        .iter()
        .enumerate()
        .map(|(index, section)| {
            format!(
                "<section class=\"section\" id=\"{}\"><div class=\"section-kicker\">{} {:02}</div><h2>{}</h2><div class=\"section-body {}\">{}</div></section>",
                id_escape(&section.section_id),
                "SECTION",
                index + 1,
                html_escape(&section.heading),
                if index == 0 { "dropcap" } else { "" },
                paragraphs(&section.body),
            )
        })
        .collect::<String>();
    let toc = if design.show_table_of_contents {
        render_toc(
            paper
                .sections
                .iter()
                .map(|section| (section.section_id.as_str(), section.heading.as_str())),
            korean,
        )
    } else {
        String::new()
    };
    let tables = paper.tables.iter().map(render_table).collect::<String>();
    let charts = paper.charts.iter().map(render_chart).collect::<String>();
    let references = if paper.references.is_empty() {
        String::new()
    } else {
        format!(
            "<section class=\"section page-break\"><div class=\"section-kicker\">REFERENCES</div><h2>{}</h2><ol class=\"reference-list\">{}</ol></section>",
            if korean { "참고문헌" } else { "References" },
            paper
                .references
                .iter()
                .map(|reference| format!("<li>{}</li>", html_escape(&reference.citation_text)))
                .collect::<String>()
        )
    };
    format!(
        "{}<section class=\"sheet content\" data-running-title=\"{}\">{}<section class=\"section\"><div class=\"section-kicker\">ABSTRACT</div><h2>{}</h2><p class=\"lead\">{}</p></section>{}{}{}{}{}{}</section>",
        cover(
            "RESEARCH PAPER",
            &paper.title,
            &paper.abstract_text,
            &[
                (if korean { "저자" } else { "Authors" }, &authors),
                (if korean { "절" } else { "Sections" }, &section_count),
                (if korean { "근거 주장" } else { "Claims" }, &claim_count),
            ],
        ),
        html_escape(&paper.title),
        toc,
        if korean { "초록" } else { "Abstract" },
        html_escape(&paper.abstract_text),
        sections,
        tables,
        charts,
        render_findings(findings, korean),
        references,
        "",
    )
}

fn render_business(
    business: &BusinessDocumentIR,
    findings: &[KnowledgeFindingIR],
    korean: bool,
    design: &DocumentDesignIR,
) -> String {
    let genre = match business.document_type {
        BusinessDocumentTypeIR::BusinessPlan => "BUSINESS PLAN",
        BusinessDocumentTypeIR::BusinessProposal => "BUSINESS PROPOSAL",
    };
    let toc = if design.show_table_of_contents {
        render_toc(
            business
                .sections
                .iter()
                .map(|section| (section.section_id.as_str(), section.heading.as_str())),
            korean,
        )
    } else {
        String::new()
    };
    let metrics = if business.key_metrics.is_empty() {
        String::new()
    } else {
        format!(
            "<div class=\"metric-grid\">{}</div>",
            business
                .key_metrics
                .iter()
                .map(|metric| format!("<article class=\"metric\"><div class=\"metric-label\">{}</div><div class=\"metric-value\">{}</div>{}</article>", html_escape(&metric.label), html_escape(&metric.value), metric.change.as_ref().map(|change| format!("<div class=\"metric-change\">{}</div>",html_escape(change))).unwrap_or_default()))
                .collect::<String>()
        )
    };
    let sections = business
        .sections
        .iter()
        .enumerate()
        .map(|(index, section)| {
            let highlights = if section.highlights.is_empty() {
                String::new()
            } else {
                format!(
                    "<div class=\"highlight-grid\">{}</div>",
                    section
                        .highlights
                        .iter()
                        .map(|item| format!("<div class=\"highlight\">{}</div>", html_escape(item)))
                        .collect::<String>()
                )
            };
            format!("<section class=\"section\" id=\"{}\"><div class=\"section-kicker\">{} {:02}</div><h2>{}</h2><div class=\"section-body\">{}</div>{}</section>", id_escape(&section.section_id), if business.document_type == BusinessDocumentTypeIR::BusinessPlan { "STRATEGY" } else { "PROPOSAL" }, index + 1, html_escape(&section.heading), paragraphs(&section.body), highlights)
        })
        .collect::<String>();
    let tables = business.tables.iter().map(render_table).collect::<String>();
    let charts = business.charts.iter().map(render_chart).collect::<String>();
    let financials = business
        .financial_statements
        .iter()
        .map(|statement| render_table(&financial_to_table(statement)))
        .collect::<String>();
    format!(
        "{}<section class=\"sheet content\" data-running-title=\"{}\">{}<section class=\"section\"><div class=\"section-kicker\">EXECUTIVE SUMMARY</div><h2>{}</h2><p class=\"lead\">{}</p>{}</section>{}{}{}{}<section class=\"section page-break\"><div class=\"section-kicker\">EXECUTION</div><h2>{}</h2>{}</section>{}{}<section class=\"cta\"><div><h3>{}</h3><p>{}</p></div><div class=\"cta-mark\">→</div></section></section>",
        cover(
            genre,
            &business.title,
            &business.executive_summary,
            &[
                (if korean { "조직" } else { "Organization" }, &business.organization),
                (if korean { "대상" } else { "Audience" }, &business.audience),
                (if korean { "핵심 지표" } else { "Key metrics" }, &business.key_metrics.len().to_string()),
            ],
        ),
        html_escape(&business.title),
        toc,
        if korean { "핵심 요약" } else { "Executive summary" },
        html_escape(&business.executive_summary),
        metrics,
        sections,
        tables,
        charts,
        financials,
        if korean { "실행 로드맵" } else { "Execution roadmap" },
        render_timeline(&business.execution_plan, korean),
        render_findings(findings, korean),
        render_risks(&business.risks, korean),
        if korean { "다음 단계" } else { "Next action" },
        html_escape(&business.next_action),
    )
}

fn render_user_guide(
    guide: &UserGuideIR,
    findings: &[KnowledgeFindingIR],
    korean: bool,
    design: &DocumentDesignIR,
) -> String {
    let toc = if design.show_table_of_contents {
        render_toc(
            guide
                .sections
                .iter()
                .map(|section| (section.section_id.as_str(), section.heading.as_str())),
            korean,
        )
    } else {
        String::new()
    };
    let sections = guide
        .sections
        .iter()
        .enumerate()
        .map(|(index, section)| {
            let steps = if section.steps.is_empty() {
                String::new()
            } else {
                format!(
                    "<ol class=\"guide-steps\">{}</ol>",
                    section
                        .steps
                        .iter()
                        .map(|step| format!("<li>{}</li>", html_escape(step)))
                        .collect::<String>()
                )
            };
            format!("<section class=\"section\" id=\"{}\"><div class=\"section-kicker\">GUIDE {:02}</div><h2>{}</h2><div class=\"section-body\">{}</div>{}</section>", id_escape(&section.section_id), index + 1, html_escape(&section.heading), paragraphs(&section.body), steps)
        })
        .collect::<Vec<_>>();
    let examples = if guide.examples.is_empty() {
        String::new()
    } else {
        format!(
            "<section class=\"section\" id=\"guide-examples\"><div class=\"section-kicker\">EXAMPLES</div><h2>{}</h2><div class=\"example-grid\">{}</div></section>",
            if korean { "바로 쓰는 예시" } else { "Ready-to-use examples" },
            guide.examples.iter().map(|example| format!("<article class=\"example-card\"><h3>{}</h3><div class=\"example-label\">{}</div><p class=\"example-copy\">{}</p><div class=\"example-label\">{}</div><p class=\"example-copy\">{}</p></article>",html_escape(&example.title),if korean{"입력"}else{"Input"},html_escape(&example.input),if korean{"기대 결과"}else{"Expected result"},html_escape(&example.expected_result))).collect::<String>()
        )
    };
    let cautions = if guide.cautions.is_empty() {
        String::new()
    } else {
        format!(
            "<aside class=\"callout\"><div class=\"callout-label\">{}</div><p>{}</p></aside>",
            if korean { "주의사항" } else { "Cautions" },
            html_escape(&guide.cautions.join(" · "))
        )
    };
    let troubleshooting = if guide.troubleshooting.is_empty() {
        String::new()
    } else {
        format!(
            "<section class=\"section\" id=\"guide-troubleshooting\"><div class=\"section-kicker\">TROUBLESHOOTING</div><h2>{}</h2><div class=\"trouble-grid\">{}</div></section>",
            if korean { "문제 해결" } else { "Troubleshooting" },
            guide.troubleshooting.iter().map(|item| format!("<article class=\"trouble-item\"><div class=\"trouble-symptom\">{}</div><div class=\"trouble-resolution\">{}</div></article>",html_escape(&item.symptom),html_escape(&item.resolution))).collect::<String>()
        )
    };
    let checklist = if guide.checklist.is_empty() {
        String::new()
    } else {
        format!(
            "<section class=\"section\" id=\"guide-checklist\"><div class=\"section-kicker\">QUICK CHECK</div><h2>{}</h2><ul class=\"checklist\">{}</ul></section>",
            if korean { "빠른 확인 목록" } else { "Quick checklist" },
            guide.checklist.iter().map(|item|format!("<li>{}</li>",html_escape(item))).collect::<String>()
        )
    };
    let tables = guide.tables.iter().map(render_table).collect::<String>();
    let charts = guide.charts.iter().map(render_chart).collect::<String>();
    let first_sections = sections.iter().take(2).cloned().collect::<String>();
    let second_sections = sections.iter().skip(2).cloned().collect::<String>();
    let page_one = document_page(
        &guide.title,
        &format!(
            "{}<section class=\"section\"><div class=\"section-kicker\">DOCUMENT PURPOSE</div><h2>{}</h2><p class=\"lead\">{}</p></section>{}",
            toc,
            if korean { "문서 목적과 적용 범위" } else { "Purpose and scope" },
            html_escape(&guide.introduction),
            first_sections,
        ),
    );
    let page_two = document_page(&guide.title, &format!("{}{}", second_sections, examples));
    let page_three = document_page(
        &guide.title,
        &format!(
            "{}{}{}{}{}{}",
            cautions,
            troubleshooting,
            checklist,
            tables,
            charts,
            render_findings(findings, korean),
        ),
    );
    format!(
        "{}{}{}{}",
        cover(
            "USER GUIDE",
            &guide.title,
            &guide.introduction,
            &[
                (if korean { "대상" } else { "Audience" }, &guide.audience),
                (
                    if korean {
                        "문서 상태"
                    } else {
                        "Document status"
                    },
                    if korean {
                        "자료 검증 전 초안"
                    } else {
                        "Draft pending source validation"
                    },
                ),
                (
                    if korean { "서식" } else { "Format" },
                    if korean {
                        "A4 전문 문서"
                    } else {
                        "Professional A4 document"
                    },
                ),
            ],
        ),
        page_one,
        page_two,
        page_three,
    )
}

fn document_page(running_title: &str, body: &str) -> String {
    format!(
        "<section class=\"sheet content\" data-running-title=\"{}\">{}</section>",
        html_escape(running_title),
        body
    )
}

fn render_table_document(table: &TableIR, findings: &[KnowledgeFindingIR], korean: bool) -> String {
    format!(
        "{}<section class=\"sheet content\" data-running-title=\"{}\">{}{}</section>",
        cover(
            "DATA TABLE",
            &table.title,
            if korean {
                "정형 데이터와 출처를 함께 표현한 표"
            } else {
                "A structured table with source-bound values"
            },
            &[
                (
                    if korean { "열" } else { "Columns" },
                    &table.columns.len().to_string()
                ),
                (
                    if korean { "행" } else { "Rows" },
                    &table.rows.len().to_string()
                ),
                (if korean { "형식" } else { "Format" }, "TABLE IR")
            ]
        ),
        html_escape(&table.title),
        render_table(table),
        render_findings(findings, korean)
    )
}

fn render_chart_document(chart: &ChartIR, findings: &[KnowledgeFindingIR], korean: bool) -> String {
    format!(
        "{}<section class=\"sheet content\" data-running-title=\"{}\">{}{}</section>",
        cover(
            "DATA VISUALIZATION",
            &chart.title,
            if korean {
                "수치 관계를 명확한 시각 문법으로 표현"
            } else {
                "Numeric relationships rendered with a clear visual grammar"
            },
            &[
                (
                    if korean { "유형" } else { "Type" },
                    &format!("{:?}", chart.chart_type)
                ),
                (
                    if korean { "계열" } else { "Series" },
                    &chart.series.len().to_string()
                ),
                (if korean { "근거" } else { "Evidence" }, "SOURCE BOUND")
            ]
        ),
        html_escape(&chart.title),
        render_chart(chart),
        render_findings(findings, korean)
    )
}

fn render_financial_document(
    statement: &FinancialStatementIR,
    findings: &[KnowledgeFindingIR],
    korean: bool,
) -> String {
    let table = financial_to_table(statement);
    format!(
        "{}<section class=\"sheet content\" data-running-title=\"{}\">{}{}</section>",
        cover(
            "FINANCIAL STATEMENT",
            &statement.entity,
            if korean {
                "기간·통화·단위를 보존한 재무 구조"
            } else {
                "Financial structure preserving periods, currency, and unit"
            },
            &[
                (
                    if korean { "유형" } else { "Type" },
                    &format!("{:?}", statement.statement_type)
                ),
                (
                    if korean { "통화" } else { "Currency" },
                    &statement.currency
                ),
                (
                    if korean { "기간" } else { "Periods" },
                    &statement.periods.len().to_string()
                )
            ]
        ),
        html_escape(&statement.entity),
        render_table(&table),
        render_findings(findings, korean)
    )
}

fn render_plan_document(
    plan: &PlanProposalIR,
    findings: &[KnowledgeFindingIR],
    korean: bool,
) -> String {
    format!("{}<section class=\"sheet content\" data-running-title=\"{}\"><section class=\"section\"><div class=\"section-kicker\">EXECUTION</div><h2>{}</h2>{}</section>{}</section>", cover("EXECUTION PLAN", &plan.title, &plan.objective, &[(if korean { "작업" } else { "Tasks" }, &plan.tasks.len().to_string()), (if korean { "위험" } else { "Risks" }, &plan.risks.len().to_string()), (if korean { "구조" } else { "Structure" }, "DEPENDENCY DAG")]), html_escape(&plan.title), if korean { "실행 단계" } else { "Execution stages" }, render_timeline(plan, korean), render_findings(findings, korean))
}

fn cover(genre: &str, title: &str, deck: &str, metadata: &[(&str, &str)]) -> String {
    format!("<section class=\"sheet cover\" data-running-title=\"{}\"><div><div class=\"eyebrow\">{}</div><h1>{}</h1><p class=\"cover-deck\">{}</p></div><div class=\"cover-meta\">{}</div></section>", html_escape(title), html_escape(genre), html_escape(title), html_escape(deck), metadata.iter().map(|(label,value)| format!("<div><div class=\"meta-label\">{}</div><div class=\"meta-value\">{}</div></div>",html_escape(label),html_escape(value))).collect::<String>())
}

fn render_toc<'a>(items: impl Iterator<Item = (&'a str, &'a str)>, korean: bool) -> String {
    format!("<nav class=\"toc\"><div class=\"toc-title\">{}</div><div class=\"toc-grid\">{}</div></nav>", if korean { "문서 구성" } else { "Document map" }, items.map(|(id,title)| format!("<a href=\"#{}\"><span>{}</span></a>",id_escape(id),html_escape(title))).collect::<String>())
}

fn render_table(table: &TableIR) -> String {
    let head = table
        .columns
        .iter()
        .map(|column| format!("<th>{}</th>", html_escape(column)))
        .collect::<String>();
    let rows = table
        .rows
        .iter()
        .map(|row| {
            format!(
                "<tr>{}</tr>",
                row.iter()
                    .map(|cell| format!(
                        "<td class=\"{}\">{}</td>",
                        if matches!(cell.value, crate::knowledge_work::CellValueIR::Number(_)) {
                            "numeric"
                        } else {
                            ""
                        },
                        html_escape(&cell.raw)
                    ))
                    .collect::<String>()
            )
        })
        .collect::<String>();
    format!("<section class=\"table-wrap\"><div class=\"table-title\">{}</div><table><thead><tr>{}</tr></thead><tbody>{}</tbody></table></section>",html_escape(&table.title),head,rows)
}

fn render_chart(chart: &ChartIR) -> String {
    let svg = super::knowledge_work::render_chart_svg(chart)
        .map(|svg| {
            svg.replace("#0b1114", "var(--paper)")
                .replace("#e8f1f2", "var(--ink)")
                .replace("#607078", "var(--line)")
                .replace("#39e6b0", "var(--accent)")
                .replace("#55c7ff", "#3d86cf")
                .replace("#ffb454", "#d78628")
                .replace("#b394ff", "#875ab9")
                .replace("#ff6f91", "#c64d70")
                .replace("#8ee36b", "#5e9f48")
        })
        .unwrap_or_else(|_| "<div>Chart data unavailable</div>".to_string());
    format!("<figure class=\"chart-card\">{}</figure>", svg)
}

fn render_timeline(plan: &PlanProposalIR, korean: bool) -> String {
    format!("<div class=\"timeline\">{}</div>", plan.tasks.iter().map(|task| format!("<article class=\"timeline-item\"><div class=\"timeline-id\">{}</div><div class=\"timeline-copy\"><h4>{}</h4><p>{}: {}{}</p></div></article>",html_escape(&task.task_id),html_escape(&task.description),if korean { "완료 조건" } else { "Completion" },html_escape(task.completion_condition.as_deref().unwrap_or(if korean { "미지정" } else { "Not specified" })),if task.dependencies.is_empty(){String::new()}else{format!(" · {}: {}",if korean{"의존"}else{"Depends on"},html_escape(&task.dependencies.join(", "))) })).collect::<String>())
}

fn render_findings(findings: &[KnowledgeFindingIR], korean: bool) -> String {
    if findings.is_empty() {
        return String::new();
    }
    format!("<section class=\"section page-break\"><div class=\"section-kicker\">EVIDENCE REVIEW</div><h2>{}</h2><div class=\"finding-list\">{}</div></section>",if korean{"근거 기반 검토"}else{"Evidence review"},findings.iter().map(|finding|format!("<article class=\"finding\"><div class=\"finding-kind\">{:?}</div><div class=\"finding-copy\">{}<br><small>{}: {}</small></div></article>",finding.kind,html_escape(&finding.statement),if korean{"근거"}else{"Evidence"},html_escape(&finding.evidence_locations.join(", ")))).collect::<String>())
}

fn render_risks(risks: &[String], korean: bool) -> String {
    if risks.is_empty() {
        return String::new();
    }
    format!(
        "<aside class=\"callout\"><div class=\"callout-label\">{}</div><p>{}</p></aside>",
        if korean {
            "위험 및 전제"
        } else {
            "Risks & assumptions"
        },
        html_escape(&risks.join(" · "))
    )
}

fn financial_to_table(statement: &FinancialStatementIR) -> TableIR {
    let mut columns = vec!["Item".to_string()];
    columns.extend(statement.periods.clone());
    let rows = statement
        .line_items
        .iter()
        .map(|item| {
            let mut row = vec![crate::knowledge_work::TableCellIR {
                value: crate::knowledge_work::CellValueIR::Text(item.label.clone()),
                raw: item.label.clone(),
                source_location: item.source_location.clone(),
            }];
            for period in &statement.periods {
                let value = item.values_by_period.get(period);
                row.push(crate::knowledge_work::TableCellIR {
                    value: value
                        .cloned()
                        .map(crate::knowledge_work::CellValueIR::Number)
                        .unwrap_or(crate::knowledge_work::CellValueIR::Missing),
                    raw: value
                        .map(|value| value.original.clone())
                        .unwrap_or_else(|| "—".to_string()),
                    source_location: item.source_location.clone(),
                });
            }
            row
        })
        .collect();
    TableIR {
        schema: crate::knowledge_work::TABLE_SCHEMA.to_string(),
        document_id: statement.document_id.clone(),
        title: format!("{} · {:?}", statement.entity, statement.statement_type),
        columns,
        rows,
        notes: Vec::new(),
    }
}

fn document_title(document: &KnowledgeDocumentIR) -> &str {
    match document {
        KnowledgeDocumentIR::Paper(value) => &value.title,
        KnowledgeDocumentIR::BusinessPlan(value) | KnowledgeDocumentIR::BusinessProposal(value) => {
            &value.title
        }
        KnowledgeDocumentIR::UserGuide(value) => &value.title,
        KnowledgeDocumentIR::Table(value) => &value.title,
        KnowledgeDocumentIR::Chart(value) => &value.title,
        KnowledgeDocumentIR::FinancialStatement(value) => &value.entity,
        KnowledgeDocumentIR::PlanProposal(value) => &value.title,
    }
}

fn paragraphs(value: &str) -> String {
    value
        .split("\n\n")
        .filter(|p| !p.trim().is_empty())
        .map(|p| format!("<p>{}</p>", html_escape(p.trim()).replace('\n', "<br>")))
        .collect()
}
fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn id_escape(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge_work::{
        BusinessDocumentIR, BusinessDocumentTypeIR, BusinessMetricIR, BusinessSectionIR,
        CellValueIR, ChartIR, ChartPointIR, ChartSeriesIR, ChartTypeIR, GuideSectionIR,
        NumericValueIR, PaperIR, PaperReferenceIR, PaperSectionIR, PlanProposalIR, PlanTaskIR,
        TableCellIR, TableIR, UserGuideIR, BUSINESS_DOCUMENT_SCHEMA, CHART_SCHEMA,
        DOCUMENT_DESIGN_SCHEMA, PAPER_SCHEMA, PLAN_PROPOSAL_SCHEMA, TABLE_SCHEMA,
        USER_GUIDE_SCHEMA,
    };

    #[test]
    fn executive_document_is_self_contained_print_ready_and_data_driven() {
        let business = BusinessDocumentIR {
            schema: BUSINESS_DOCUMENT_SCHEMA.to_string(),
            document_id: "B-1".to_string(),
            document_type: BusinessDocumentTypeIR::BusinessPlan,
            title: "North Star 2027".to_string(),
            organization: "Orbit Labs".to_string(),
            audience: "Investment committee".to_string(),
            executive_summary: "A bounded expansion plan.".to_string(),
            sections: vec![BusinessSectionIR {
                section_id: "S-1".to_string(),
                heading: "Opportunity".to_string(),
                body: "Verified demand signal.".to_string(),
                highlights: vec!["Low acquisition cost".to_string()],
            }],
            key_metrics: vec![BusinessMetricIR {
                label: "ARR".to_string(),
                value: "$2.4M".to_string(),
                change: Some("+38% YoY".to_string()),
                evidence_location: "table:1".to_string(),
            }],
            execution_plan: PlanProposalIR {
                schema: PLAN_PROPOSAL_SCHEMA.to_string(),
                document_id: "P-1".to_string(),
                title: "Roadmap".to_string(),
                objective: "Scale".to_string(),
                tasks: vec![PlanTaskIR {
                    task_id: "Q1".to_string(),
                    description: "Validate".to_string(),
                    dependencies: Vec::new(),
                    owner: None,
                    completion_condition: Some("gate passes".to_string()),
                }],
                risks: Vec::new(),
                assumptions: Vec::new(),
            },
            tables: vec![TableIR {
                schema: TABLE_SCHEMA.to_string(),
                document_id: "T-1".to_string(),
                title: "Unit economics".to_string(),
                columns: vec!["Metric".to_string(), "Value".to_string()],
                rows: vec![vec![
                    TableCellIR {
                        value: CellValueIR::Text("Gross margin".to_string()),
                        raw: "Gross margin".to_string(),
                        source_location: "model:margin".to_string(),
                    },
                    TableCellIR {
                        value: CellValueIR::Number(NumericValueIR {
                            coefficient: 72,
                            scale: 0,
                            unit: Some("%".to_string()),
                            original: "72%".to_string(),
                        }),
                        raw: "72%".to_string(),
                        source_location: "model:margin".to_string(),
                    },
                ]],
                notes: Vec::new(),
            }],
            charts: vec![ChartIR {
                schema: CHART_SCHEMA.to_string(),
                document_id: "C-1".to_string(),
                title: "Revenue trajectory".to_string(),
                chart_type: ChartTypeIR::Line,
                category_axis: "Quarter".to_string(),
                value_axis: "ARR".to_string(),
                series: vec![ChartSeriesIR {
                    name: "ARR".to_string(),
                    points: vec![
                        ChartPointIR {
                            category: "Q1".to_string(),
                            value: Some(NumericValueIR {
                                coefficient: 14,
                                scale: 0,
                                unit: Some("M".to_string()),
                                original: "14".to_string(),
                            }),
                            source_location: "forecast:Q1".to_string(),
                        },
                        ChartPointIR {
                            category: "Q2".to_string(),
                            value: Some(NumericValueIR {
                                coefficient: 24,
                                scale: 0,
                                unit: Some("M".to_string()),
                                original: "24".to_string(),
                            }),
                            source_location: "forecast:Q2".to_string(),
                        },
                    ],
                }],
            }],
            financial_statements: Vec::new(),
            risks: vec!["Demand may vary".to_string()],
            next_action: "Approve pilot".to_string(),
        };
        let design = DocumentDesignIR {
            schema: DOCUMENT_DESIGN_SCHEMA.to_string(),
            theme: DocumentThemeIR::ExecutiveNavy,
            page_size: PageSizeIR::A4,
            brand_name: Some("Orbit Labs".to_string()),
            accent_color: Some("#0A8F7A".to_string()),
            compact: false,
            show_table_of_contents: true,
            show_page_furniture: true,
        };
        let html = render_print_ready_html(
            &KnowledgeDocumentIR::BusinessPlan(business),
            &[],
            LanguageCodeIR::English,
            &design,
        );
        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("@page { size: A4"));
        assert!(html.contains("$2.4M"));
        assert!(html.contains("+38% YoY"));
        assert!(html.contains("--accent:#0A8F7A"));
        assert!(html.contains("class=\"timeline\""));
        assert!(html.contains("Revenue trajectory"));
        assert!(html.contains("fill=\"var(--paper)\""));
        assert!(html.contains("Unit economics"));
        assert!(html.contains("class=\"numeric\">72%"));
    }

    #[test]
    fn academic_document_uses_editorial_contract_and_preserves_authorship() {
        let paper = PaperIR {
            schema: PAPER_SCHEMA.to_string(),
            document_id: "P-ACADEMIC".to_string(),
            title: "Bounded Semantic Composition".to_string(),
            authors: vec!["Gray K.".to_string(), "B_CORE Lab".to_string()],
            abstract_text: "A source-bound evaluation of semantic composition.".to_string(),
            sections: vec![PaperSectionIR {
                section_id: "methods".to_string(),
                heading: "Methods".to_string(),
                body: "The evaluation preserves typed evidence boundaries.".to_string(),
                level: 2,
            }],
            claims: Vec::new(),
            references: vec![PaperReferenceIR {
                reference_id: "R-1".to_string(),
                citation_text: "Verified source record.".to_string(),
            }],
            tables: Vec::new(),
            charts: Vec::new(),
        };
        let design = DocumentDesignIR::for_kind(crate::knowledge_work::DocumentKindIR::Paper);
        let html = render_print_ready_html(
            &KnowledgeDocumentIR::Paper(paper),
            &[],
            LanguageCodeIR::English,
            &design,
        );
        assert!(html.contains("class=\"theme-academic\""));
        assert!(html.contains("Gray K., B_CORE Lab"));
        assert!(html.contains("Document map"));
        assert!(html.contains("References"));
        assert!(html.contains("@media print"));
    }

    #[test]
    fn user_guide_preserves_required_section_order_and_source_content() {
        let guide = UserGuideIR {
            schema: USER_GUIDE_SCHEMA.to_string(),
            document_id: "GUIDE-FORMAT-CONTRACT".to_string(),
            title: "운영 절차서".to_string(),
            audience: "승인 담당자".to_string(),
            introduction: "승인된 절차와 증빙 기준을 정의한다.".to_string(),
            sections: vec![
                GuideSectionIR {
                    section_id: "required-b".to_string(),
                    heading: "02 검토 및 승인".to_string(),
                    body: "검토자는 필수 증빙과 승인 조건을 대조한다.".to_string(),
                    steps: vec!["승인 조건 충족 여부를 기록한다.".to_string()],
                },
                GuideSectionIR {
                    section_id: "required-a".to_string(),
                    heading: "01 접수".to_string(),
                    body: "접수 원문을 수정하지 않고 등록한다.".to_string(),
                    steps: vec!["원문 식별자를 부여한다.".to_string()],
                },
            ],
            examples: Vec::new(),
            cautions: Vec::new(),
            troubleshooting: Vec::new(),
            checklist: Vec::new(),
            tables: Vec::new(),
            charts: Vec::new(),
        };
        let design = DocumentDesignIR::for_kind(crate::knowledge_work::DocumentKindIR::UserGuide);
        let html = render_print_ready_html(
            &KnowledgeDocumentIR::UserGuide(guide),
            &[],
            LanguageCodeIR::Korean,
            &design,
        );

        let review_position = html.find("02 검토 및 승인").expect("required section B");
        let intake_position = html.find("01 접수").expect("required section A");
        assert!(review_position < intake_position);
        assert!(html.contains("검토자는 필수 증빙과 승인 조건을 대조한다."));
        assert!(html.contains("접수 원문을 수정하지 않고 등록한다."));
        assert!(html.contains("승인 조건 충족 여부를 기록한다."));
        assert!(html.contains("원문 식별자를 부여한다."));
        assert_eq!(html.matches("class=\"sheet").count(), 4);
    }
}
