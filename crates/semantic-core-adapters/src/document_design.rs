use crate::{
    knowledge_work::{
        BusinessDocumentIR, BusinessDocumentTypeIR, ChartIR, DocumentDesignIR, DocumentThemeIR,
        FinancialStatementIR, KnowledgeDocumentIR, KnowledgeFindingIR, PageSizeIR, PaperIR,
        PlanProposalIR, TableIR,
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
        KnowledgeDocumentIR::Table(table) => render_table_document(table, findings, korean),
        KnowledgeDocumentIR::Chart(chart) => render_chart_document(chart, findings, korean),
        KnowledgeDocumentIR::FinancialStatement(statement) => {
            render_financial_document(statement, findings, korean)
        }
        KnowledgeDocumentIR::PlanProposal(plan) => render_plan_document(plan, findings, korean),
    };
    let brand = design
        .brand_name
        .as_deref()
        .map(html_escape)
        .unwrap_or_else(|| "B_CORE • EVIDENCE-BOUND DOCUMENT".to_string());
    format!(
        "<!doctype html><html lang=\"{}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{}</title><style>{}</style></head><body class=\"{}\"><div class=\"ambient ambient-a\"></div><div class=\"ambient ambient-b\"></div><header class=\"running-head\"><span>{}</span><span class=\"running-title\">{}</span></header><main class=\"document-shell\">{}</main><footer class=\"running-foot\"><span>{}</span><span class=\"page-number\"></span></footer></body></html>",
        if korean { "ko" } else { "en" },
        html_escape(title),
        stylesheet(&tokens, design),
        tokens.theme_class,
        brand,
        html_escape(title),
        body,
        if korean { "검증 가능한 근거에 기반한 문서" } else { "Evidence-bound document" },
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
    let page = match design.page_size {
        PageSizeIR::A4 => "A4",
        PageSizeIR::Letter => "Letter",
    };
    let section_gap = if design.compact { "24px" } else { "42px" };
    let furniture = if design.show_page_furniture {
        "display:flex"
    } else {
        "display:none"
    };
    format!(
        r#"
@page {{ size: {page}; margin: 18mm 17mm 20mm; @bottom-right {{ content: counter(page); }} }}
:root {{ --ink:{ink}; --muted:{muted}; --paper:{paper}; --surface:{surface}; --accent:{accent}; --accent-soft:{accent_soft}; --line:{line}; --display:{display}; --body:{body}; --section-gap:{section_gap}; }}
* {{ box-sizing:border-box; }}
html {{ background:#dfe4e7; color:var(--ink); font-family:var(--body); font-size:15px; line-height:1.72; text-rendering:optimizeLegibility; }}
body {{ margin:0; min-height:100vh; background:linear-gradient(140deg,#dfe4e7,#f0f2f3 48%,#d7dde1); }}
.ambient {{ position:fixed; border-radius:50%; filter:blur(70px); opacity:.22; pointer-events:none; }}
.ambient-a {{ width:420px;height:420px;background:var(--accent);top:-240px;right:-100px; }}
.ambient-b {{ width:320px;height:320px;background:var(--accent);bottom:-230px;left:-120px;opacity:.12; }}
.document-shell {{ width:min(1120px,calc(100% - 48px)); margin:58px auto 90px; background:var(--paper); box-shadow:0 24px 70px rgba(17,32,47,.16),0 2px 7px rgba(17,32,47,.08); border:1px solid rgba(255,255,255,.78); min-height:80vh; overflow:hidden; position:relative; }}
.running-head,.running-foot {{ position:fixed; z-index:20; left:50%; transform:translateX(-50%); width:min(1060px,calc(100% - 80px)); color:var(--muted); text-transform:uppercase; letter-spacing:.13em; font:600 10px/1 var(--body); {furniture}; justify-content:space-between; pointer-events:none; }}
.running-head {{ top:22px; }} .running-foot {{ bottom:20px; }}
.cover {{ min-height:680px; padding:76px 78px 64px; display:flex; flex-direction:column; justify-content:space-between; position:relative; isolation:isolate; overflow:hidden; }}
.cover::before {{ content:''; position:absolute; inset:0; background:linear-gradient(120deg,var(--paper) 0 62%,var(--accent-soft) 62%); z-index:-2; }}
.cover::after {{ content:''; position:absolute; width:430px;height:430px;border:90px solid var(--accent);border-radius:50%;right:-210px;top:-140px;opacity:.95;z-index:-1; }}
.eyebrow {{ display:inline-flex; align-items:center; gap:10px; color:var(--accent); text-transform:uppercase; letter-spacing:.18em; font:700 11px/1 var(--body); }}
.eyebrow::before {{ content:'';width:34px;height:2px;background:var(--accent); }}
.cover h1 {{ max-width:780px; margin:24px 0 18px; font:700 clamp(45px,6vw,76px)/.98 var(--display); letter-spacing:-.045em; text-wrap:balance; }}
.cover-deck {{ max-width:660px; color:var(--muted); font-size:19px; line-height:1.55; }}
.cover-meta {{ display:grid; grid-template-columns:repeat(3,minmax(0,1fr)); gap:22px; padding-top:28px; border-top:1px solid var(--line); }}
.meta-label {{ color:var(--muted); text-transform:uppercase;letter-spacing:.12em;font-size:10px;font-weight:700; }}
.meta-value {{ margin-top:5px;font:650 14px/1.35 var(--body); }}
.content {{ padding:64px 78px 80px; }}
.toc {{ margin:0 0 62px; padding:30px 34px; border:1px solid var(--line); background:linear-gradient(135deg,var(--surface),transparent); break-inside:avoid; }}
.toc-title {{ margin:0 0 18px; color:var(--accent);font:700 11px/1 var(--body);letter-spacing:.16em;text-transform:uppercase; }}
.toc-grid {{ display:grid; grid-template-columns:repeat(2,minmax(0,1fr));gap:8px 28px;counter-reset:toc; }}
.toc a {{ color:var(--ink);text-decoration:none;border-bottom:1px dotted var(--line);padding:7px 0;display:flex;justify-content:space-between;gap:14px; }}
.toc a::before {{ counter-increment:toc;content:counter(toc,decimal-leading-zero);color:var(--accent);font-weight:700; }}
.section {{ margin-top:var(--section-gap); break-inside:auto; }}
.section-kicker {{ color:var(--accent);font:700 10px/1 var(--body);letter-spacing:.16em;text-transform:uppercase;margin-bottom:9px; }}
.section h2 {{ margin:0 0 18px;font:700 34px/1.12 var(--display);letter-spacing:-.025em; }}
.section h3 {{ margin:26px 0 10px;font:700 22px/1.22 var(--display); }}
.section-body {{ color:var(--ink);white-space:pre-wrap;max-width:76ch; }}
.lead {{ font-size:20px;line-height:1.55;color:var(--ink);max-width:75ch; }}
.dropcap::first-letter {{ float:left;font:700 64px/.78 var(--display);color:var(--accent);padding:9px 10px 0 0; }}
.metric-grid {{ display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:14px;margin:28px 0 36px; }}
.metric {{ background:var(--surface);padding:22px 20px;border-top:4px solid var(--accent);break-inside:avoid; }}
.metric-label {{ color:var(--muted);font-size:11px;font-weight:700;letter-spacing:.08em;text-transform:uppercase; }}
.metric-value {{ margin:12px 0 3px;font:700 31px/1 var(--display);letter-spacing:-.03em; }}
.metric-change {{ color:var(--accent);font-size:12px;font-weight:700; }}
.highlight-grid {{ display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:16px;margin-top:22px; }}
.highlight {{ padding:20px 22px;background:var(--surface);border-left:3px solid var(--accent);font-weight:600;break-inside:avoid; }}
.callout {{ margin:32px 0;padding:28px 30px;background:var(--ink);color:var(--paper);position:relative;overflow:hidden;break-inside:avoid; }}
.callout::after {{ content:'';position:absolute;width:130px;height:130px;border:30px solid var(--accent);border-radius:50%;right:-68px;top:-70px; }}
.callout-label {{ color:var(--accent-soft);text-transform:uppercase;letter-spacing:.14em;font-size:10px;font-weight:800; }}
.callout p {{ margin:9px 0 0;font:650 20px/1.45 var(--display);max-width:80%; }}
.table-wrap {{ margin:28px 0 38px;overflow:hidden;border:1px solid var(--line);break-inside:avoid; }}
.table-title {{ padding:16px 20px;background:var(--ink);color:var(--paper);font:700 14px/1.2 var(--display); }}
table {{ width:100%;border-collapse:collapse;font-variant-numeric:tabular-nums; }}
th {{ padding:13px 16px;background:var(--accent-soft);color:var(--ink);text-align:left;font-size:11px;letter-spacing:.06em;text-transform:uppercase;border-bottom:1px solid var(--line); }}
td {{ padding:13px 16px;border-bottom:1px solid var(--line);vertical-align:top; }}
tr:nth-child(even) td {{ background:color-mix(in srgb,var(--surface) 55%,transparent); }}
td.numeric {{ text-align:right;font-weight:700; }}
.chart-card {{ margin:28px 0 42px;padding:22px;background:var(--surface);border:1px solid var(--line);break-inside:avoid; }}
.chart-card svg {{ display:block;width:100%;height:auto;max-height:520px; }}
.timeline {{ display:grid;gap:0;margin:30px 0; }}
.timeline-item {{ display:grid;grid-template-columns:94px 1fr;gap:22px;padding:0 0 28px;position:relative; }}
.timeline-item::before {{ content:'';position:absolute;left:45px;top:26px;bottom:-2px;width:1px;background:var(--line); }}
.timeline-item:last-child::before {{ display:none; }}
.timeline-id {{ width:90px;height:30px;display:grid;place-items:center;background:var(--accent);color:white;font:800 10px/1 var(--body);letter-spacing:.07em;z-index:1; }}
.timeline-copy h4 {{ margin:1px 0 7px;font:700 17px/1.25 var(--display); }}
.timeline-copy p {{ margin:0;color:var(--muted);font-size:13px; }}
.finding-list {{ display:grid;gap:10px;margin-top:18px; }}
.finding {{ display:grid;grid-template-columns:110px 1fr;gap:18px;padding:15px 0;border-bottom:1px solid var(--line); }}
.finding-kind {{ color:var(--accent);font-size:10px;font-weight:800;letter-spacing:.08em;text-transform:uppercase; }}
.finding-copy {{ font-size:13px; }}
.reference-list {{ padding-left:20px;color:var(--muted);font-size:13px; }}
.page-break {{ break-before:page; }}
.cta {{ margin-top:48px;padding:38px 42px;background:linear-gradient(120deg,var(--ink),color-mix(in srgb,var(--ink) 82%,var(--accent)));color:white;display:grid;grid-template-columns:1fr auto;gap:32px;align-items:center;break-inside:avoid; }}
.cta h3 {{ margin:0 0 8px;font:700 28px/1.12 var(--display); }} .cta p {{ margin:0;color:#dce4ec;max-width:62ch; }}
.cta-mark {{ width:54px;height:54px;border-radius:50%;display:grid;place-items:center;background:var(--accent);font-size:25px; }}
.theme-academic .cover::before {{ background:linear-gradient(115deg,var(--paper) 0 70%,var(--accent-soft) 70%); }}
.theme-academic .cover h1 {{ font-weight:600; }}
.theme-academic .content {{ max-width:920px;margin:auto; }}
.theme-proposal .cover::before {{ background:linear-gradient(125deg,var(--ink) 0 57%,var(--accent) 57%); }}
.theme-proposal .cover h1,.theme-proposal .cover-deck,.theme-proposal .cover .meta-value {{ color:white; }}
.theme-proposal .cover .eyebrow,.theme-proposal .cover .meta-label {{ color:#bcd0ff; }}
.theme-proposal .cover::after {{ border-color:white;opacity:.1; }}
@media (max-width:760px) {{ .document-shell {{ width:100%;margin:0;box-shadow:none; }} .cover,.content {{ padding:44px 26px; }} .cover {{ min-height:620px; }} .cover-meta,.metric-grid,.highlight-grid,.toc-grid {{ grid-template-columns:1fr 1fr; }} .running-head,.running-foot {{ display:none; }} }}
@media print {{ html,body {{ background:white; }} .ambient,.running-head,.running-foot {{ display:none; }} .document-shell {{ width:auto;margin:0;box-shadow:none;border:0; }} .cover {{ min-height:250mm;break-after:page; }} .content {{ padding:0; }} .chart-card,.table-wrap,.metric,.callout,.cta {{ break-inside:avoid; }} a {{ color:inherit; }} }}
"#,
        page = page,
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
        furniture = furniture,
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
        "{}<div class=\"content\">{}<section class=\"section\"><div class=\"section-kicker\">ABSTRACT</div><h2>{}</h2><p class=\"lead\">{}</p></section>{}{}{}{}{}{}</div>",
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
        "{}<div class=\"content\">{}<section class=\"section\"><div class=\"section-kicker\">EXECUTIVE SUMMARY</div><h2>{}</h2><p class=\"lead\">{}</p>{}</section>{}{}{}{}<section class=\"section page-break\"><div class=\"section-kicker\">EXECUTION</div><h2>{}</h2>{}</section>{}{}<section class=\"cta\"><div><h3>{}</h3><p>{}</p></div><div class=\"cta-mark\">→</div></section></div>",
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

fn render_table_document(table: &TableIR, findings: &[KnowledgeFindingIR], korean: bool) -> String {
    format!(
        "{}<div class=\"content\">{}{}</div>",
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
        render_table(table),
        render_findings(findings, korean)
    )
}

fn render_chart_document(chart: &ChartIR, findings: &[KnowledgeFindingIR], korean: bool) -> String {
    format!(
        "{}<div class=\"content\">{}{}</div>",
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
        "{}<div class=\"content\">{}{}</div>",
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
        render_table(&table),
        render_findings(findings, korean)
    )
}

fn render_plan_document(
    plan: &PlanProposalIR,
    findings: &[KnowledgeFindingIR],
    korean: bool,
) -> String {
    format!("{}<div class=\"content\"><section class=\"section\"><div class=\"section-kicker\">EXECUTION</div><h2>{}</h2>{}</section>{}</div>", cover("EXECUTION PLAN", &plan.title, &plan.objective, &[(if korean { "작업" } else { "Tasks" }, &plan.tasks.len().to_string()), (if korean { "위험" } else { "Risks" }, &plan.risks.len().to_string()), (if korean { "구조" } else { "Structure" }, "DEPENDENCY DAG")]), if korean { "실행 단계" } else { "Execution stages" }, render_timeline(plan, korean), render_findings(findings, korean))
}

fn cover(genre: &str, title: &str, deck: &str, metadata: &[(&str, &str)]) -> String {
    format!("<section class=\"cover\"><div><div class=\"eyebrow\">{}</div><h1>{}</h1><p class=\"cover-deck\">{}</p></div><div class=\"cover-meta\">{}</div></section>", html_escape(genre), html_escape(title), html_escape(deck), metadata.iter().map(|(label,value)| format!("<div><div class=\"meta-label\">{}</div><div class=\"meta-value\">{}</div></div>",html_escape(label),html_escape(value))).collect::<String>())
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
        CellValueIR, ChartIR, ChartPointIR, ChartSeriesIR, ChartTypeIR, NumericValueIR, PaperIR,
        PaperReferenceIR, PaperSectionIR, PlanProposalIR, PlanTaskIR, TableCellIR, TableIR,
        BUSINESS_DOCUMENT_SCHEMA, CHART_SCHEMA, DOCUMENT_DESIGN_SCHEMA, PAPER_SCHEMA,
        PLAN_PROPOSAL_SCHEMA, TABLE_SCHEMA,
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
}
