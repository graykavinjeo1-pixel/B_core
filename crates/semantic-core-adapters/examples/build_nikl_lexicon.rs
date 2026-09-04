//! Reproducible, offline text-only import of the NIKL bulk JSON download.
//! Usage: cargo run -p semantic-core-adapters --example build_nikl_lexicon -- INPUT_DIR OUTPUT_JSONL
//! No examples, audio, images or inferred translations are imported.
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

fn items(value: &Value) -> Vec<&Value> {
    match value {
        Value::Array(values) => values.iter().collect(),
        Value::Object(_) => vec![value],
        _ => vec![],
    }
}
fn feature(value: &Value, name: &str) -> String {
    items(&value["feat"])
        .into_iter()
        .find(|v| v["att"] == name)
        .and_then(|v| v["val"].as_str())
        .unwrap_or("")
        .trim()
        .to_string()
}
fn features(value: &Value) -> BTreeMap<String, String> {
    items(&value["feat"])
        .into_iter()
        .filter_map(|v| Some((v["att"].as_str()?.into(), v["val"].as_str()?.trim().into())))
        .collect()
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.get(1).is_some_and(|arg| arg == "--select") {
        if args.len() != 4 {
            return Err("expected --select INPUT_JSONL OUTPUT_JSONL".into());
        }
        return select_pack(&args[2], &args[3]);
    }
    if args.len() != 3 {
        return Err("expected input directory and output JSONL".into());
    }
    let mut files = fs::read_dir(&args[1])?
        .map(|e| e.map(|e| e.path()))
        .collect::<Result<Vec<_>, _>>()?;
    files.retain(|p| p.extension().is_some_and(|e| e == "json"));
    files.sort();
    let mut output = BufWriter::new(fs::File::create(Path::new(&args[2]))?);
    let mut categories = BTreeMap::<String, usize>::new();
    let mut levels = BTreeMap::<String, usize>::new();
    let mut positions = BTreeMap::<String, usize>::new();
    let mut total = 0;
    let mut bilingual = 0;
    for file in files {
        let resource: Value = serde_json::from_slice(&fs::read(&file)?)?;
        for entry in items(&resource["LexicalResource"]["Lexicon"]["LexicalEntry"]) {
            total += 1;
            let senses = items(&entry["Sense"]).into_iter().filter_map(|sense| {
                let equivalent = items(&sense["Equivalent"]).into_iter().find(|e| feature(e, "language") == "영어")?;
                let en = feature(equivalent, "lemma");
                let def_ko = feature(sense, "definition");
                let def_en = feature(equivalent, "definition");
                if en.is_empty() || def_ko.is_empty() || def_en.is_empty() {return None;}
                Some(json!({"source_sense_id":sense["val"],"english":en,"definition_ko":def_ko,"definition_en":def_en,
                    "grammar":features(sense),"frames":items(&sense["SyntacticBehaviour"]).into_iter().map(features).collect::<Vec<_>>() }))
            }).collect::<Vec<_>>();
            if senses.is_empty() {
                continue;
            }
            bilingual += 1;
            let attrs = features(entry);
            let pos = feature(entry, "partOfSpeech");
            let level = feature(entry, "vocabularyLevel");
            *positions.entry(pos.clone()).or_default() += 1;
            *levels.entry(level.clone()).or_default() += 1;
            for (key, value) in &attrs {
                if key.to_lowercase().contains("category") {
                    *categories.entry(format!("{key}:{value}")).or_default() += 1;
                }
            }
            let forms = items(&entry["WordForm"])
                .into_iter()
                .filter(|f| feature(f, "type") != "발음")
                .map(|f| {
                    let mut attrs = features(f);
                    attrs.remove("sound");
                    attrs
                })
                .collect::<Vec<_>>();
            serde_json::to_writer(
                &mut output,
                &json!({"source_entry_id":entry["val"],"lemma":feature(&entry["Lemma"],"writtenForm"),
                "pos":pos,"level":level,"attributes":attrs,"forms":forms,"senses":senses}),
            )?;
            output.write_all(b"\n")?;
        }
        eprintln!("processed {}", file.file_name().unwrap().to_string_lossy());
    }
    output.flush()?;
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"entries":total,"bilingual_entries":bilingual,"levels":levels,"pos":positions,"categories":categories})
        )?
    );
    Ok(())
}

fn domain_evidence(row: &Value) -> Vec<String> {
    let category = row["attributes"]["semanticCategory"].as_str().unwrap_or("");
    let subject = row["attributes"]["subjectCategiory"].as_str().unwrap_or("");
    if category.starts_with("경제 생활")
        || category.contains("사법 및 치안")
        || matches!(subject, "법" | "경제·경영")
    {
        return vec![format!("NIKL_CATEGORY:{category}:{subject}")];
    }
    if matches!(
        category,
        "사회 생활 > 직업" | "사회 생활 > 직위" | "사회 생활 > 직장" | "사회 생활 > 직장 생활"
    ) {
        return vec![format!("NIKL_EMPLOYMENT_CATEGORY:{category}")];
    }
    // Lexical definitions identify related-domain candidates, NOT legal conclusions.
    // Avoid word-sense traps such as river banks, soup stock and physical properties.
    if [
        "자연 >",
        "동식물 >",
        "문화 >",
        "종교 >",
        "식생활 >",
        "개념 > 지역",
    ]
    .iter()
    .any(|c| category.starts_with(c))
    {
        return vec![];
    }
    let terms = "legal judicial legislation legislative lawsuit prosecution prosecutor statute criminal defendant plaintiff attorney lawyer imprisonment punishment contract liability patent copyright ownership finance financial monetary economic economy commerce commercial business enterprise company corporation banking loan debt credit investment investor shareholder revenue profit income wage salary employment employee employer labor tax taxation budget insurance currency money payment price purchase sale trade export import market rent lease".split_whitespace().collect::<BTreeSet<_>>();
    let mut evidence = BTreeSet::new();
    for sense in items(&row["senses"]) {
        let gloss = sense["definition_en"].as_str().unwrap_or("").to_lowercase();
        for word in gloss.split(|c: char| !c.is_ascii_alphabetic()) {
            if terms.contains(word) {
                evidence.insert(format!(
                    "NIKL_DEFINITION:{}:{word}",
                    sense["source_sense_id"].as_str().unwrap_or("")
                ));
            }
        }
    }
    evidence.into_iter().collect()
}

fn level_rank(row: &Value) -> u8 {
    match row["level"].as_str().unwrap_or("") {
        "초급" => 0,
        "중급" => 1,
        "고급" => 2,
        _ => 3,
    }
}

fn select_pack(input: &str, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut groups = BTreeMap::<String, Vec<Value>>::new();
    let mut grammar = Vec::new();
    for line in BufReader::new(fs::File::open(input)?).lines() {
        let mut row: Value = serde_json::from_str(&line?)?;
        let senses = row["senses"].as_array_mut().ok_or("missing senses")?;
        senses.retain(|s| {
            let en = s["english"].as_str().unwrap_or("").to_lowercase();
            !en.is_empty()
                && !en.contains("no equivalent")
                && !en.contains("no corresponding")
                && !en.contains("no matching")
        });
        if senses.is_empty() {
            continue;
        }
        let lemma = row["lemma"].as_str().unwrap_or("").to_string();
        if lemma.is_empty() || lemma.len() > 160 {
            continue;
        }
        if matches!(
            row["attributes"]["lexicalUnit"].as_str(),
            Some("문법‧표현" | "문법·표현")
        ) {
            grammar.push(row);
            continue;
        }
        match row["pos"].as_str().unwrap_or("") {
            "어미" | "조사" | "접사" => grammar.push(row),
            "품사 없음" if row["attributes"]["lexicalUnit"] == "구" => {
                groups.entry(lemma).or_default().push(row)
            }
            "명사" | "동사" | "형용사" | "부사" | "관형사" | "대명사" | "감탄사" | "수사"
            | "의존 명사" | "보조 동사" | "보조 형용사" => {
                groups.entry(lemma).or_default().push(row)
            }
            _ => {}
        }
    }
    let mut domain = groups
        .iter()
        .filter_map(|(lemma, rows)| {
            let evidence = rows
                .iter()
                .flat_map(domain_evidence)
                .collect::<BTreeSet<_>>();
            if evidence.is_empty() {
                return None;
            }
            let tier = if evidence.iter().any(|e| e.starts_with("NIKL_CATEGORY:")) {
                0
            } else if evidence.iter().any(|e| e.starts_with("NIKL_DEFINITION:")) {
                1
            } else {
                2
            };
            Some((
                tier,
                rows.iter().map(level_rank).min().unwrap_or(3),
                lemma.clone(),
            ))
        })
        .collect::<Vec<_>>();
    domain.sort();
    if domain.len() < 5_000 {
        return Err(format!(
            "only {} defensible domain lemma candidates; do not pad",
            domain.len()
        )
        .into());
    }
    let domain_candidates = domain.len();
    let selected_domain = domain
        .into_iter()
        .take(5_000)
        .map(|(_, _, l)| l)
        .collect::<BTreeSet<_>>();
    let mut general = groups
        .iter()
        .filter(|(l, _)| !selected_domain.contains(*l))
        .map(|(l, rows)| {
            // Learner grade is a frequency proxy, not a measured corpus frequency.
            let grade = rows.iter().map(level_rank).min().unwrap_or(3);
            let conversational = rows.iter().any(|r| {
                matches!(
                    r["pos"].as_str(),
                    Some("대명사" | "감탄사" | "부사" | "형용사")
                )
            });
            // User-requested lexical coverage anchors, never utterance/answer templates.
            let requested = matches!(
                l.as_str(),
                "애매하다" | "귀찮다" | "답답하다" | "아쉽다" | "솔직히" | "글쎄" | "어쩐지"
            );
            (!requested, grade, !conversational, l.clone())
        })
        .collect::<Vec<_>>();
    general.sort();
    if general.len() < 10_000 {
        return Err("not enough general lemmas".into());
    }
    let selected_general = general
        .into_iter()
        .take(10_000)
        .map(|(_, _, _, l)| l)
        .collect::<BTreeSet<_>>();
    let mut selected = Vec::new();
    for (lemma, rows) in groups {
        let domain = if selected_domain.contains(&lemma) {
            "LAW_ECONOMICS_RELATED"
        } else if selected_general.contains(&lemma) {
            "GENERAL"
        } else {
            continue;
        };
        for mut row in rows {
            row["selection_evidence"] = json!(if domain == "GENERAL" {
                vec![format!(
                    "NIKL_LEARNER_LEVEL:{}",
                    row["level"].as_str().unwrap_or("")
                )]
            } else {
                domain_evidence(&row)
            });
            row["domain"] = json!(domain);
            selected.push(row);
        }
    }
    // Grammar is supplementary and never counted toward the 15,000-word target.
    for mut row in grammar {
        row["domain"] = json!("GRAMMAR");
        row["selection_evidence"] = json!(["NIKL_GRAMMAR_ENTRY"]);
        selected.push(row);
    }
    selected.sort_by_key(|r| r["source_entry_id"].as_str().unwrap_or("").to_string());
    // Export only lexical facts, source-linked sense definitions and grammar.
    let mut writer = BufWriter::new(fs::File::create(output)?);
    let mut counts = BTreeMap::<String, usize>::new();
    let mut senses = 0;
    for row in &mut selected {
        *counts
            .entry(row["domain"].as_str().unwrap().into())
            .or_default() += 1;
        for sense in row["senses"].as_array_mut().unwrap() {
            senses += 1;
            sense["grammar"]
                .as_object_mut()
                .unwrap()
                .remove("definition");
        }
        serde_json::to_writer(&mut writer, row)?;
        writer.write_all(b"\n")?;
    }
    writer.flush()?;
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"general_unique_lemmas":selected_general.len(),"law_economics_related_unique_lemmas":selected_domain.len(),"domain_candidate_lemmas":domain_candidates,"source_entries_by_partition":counts,"bilingual_senses":senses,"selection_warning":"Learner-grade prioritized general vocabulary; law/economics-related vocabulary includes employment and source-definition-based candidates, not 5,000 independently expert-reviewed technical terms."})
        )?
    );
    Ok(())
}
