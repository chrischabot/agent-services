use super::*;

impl Store {
    pub(crate) fn insert_x_item(&self, input: XItemInput) -> Result<Option<XItem>> {
        validate_x_item_input(&input)?;
        let x_author_id = input
            .source_metadata
            .get("x_author_id")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty());
        resolve_x_profile_id_on(&self.conn, &input.author, x_author_id, &input)?;
        let existing: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM x_items WHERE x_id = ?1",
                params![input.x_id],
                |row| row.get(0),
            )
            .optional()?;
        if existing.is_some() {
            self.update_existing_x_item(&input)?;
            self.upsert_x_item_source(&input)?;
            upsert_x_canonical_on(&self.conn, &input, None, None)?;
            return Ok(None);
        }

        let retrieved_at = input.retrieved_at.clone().unwrap_or_else(now);
        let metrics_json = canonical_json(&input.metrics)?;
        let raw_json = canonical_json(&input.raw)?;
        let card = self.add_source_card(SourceCardInput {
            title: format!("X: {} {}", input.author, input.x_id),
            url: input.url.clone(),
            source_type: "x".to_string(),
            provider: "x".to_string(),
            summary: input.text.clone(),
            claims: vec![SourceClaim {
                claim: input.text.clone(),
                kind: "source_text".to_string(),
                confidence: 1.0,
            }],
            retrieved_at: Some(retrieved_at.clone()),
            metadata: json!({
                "x_id": input.x_id,
                "author": input.author,
                "created_at": input.created_at,
                "source_kind": input.source_kind,
                "source_detail": input.source_detail,
                "metrics": input.metrics
            }),
        })?;
        let id = Uuid::new_v4().to_string();
        let imported_at = now();
        self.conn.execute(
            r#"
            INSERT INTO x_items
              (id, x_id, author, text, url, created_at, imported_at, retrieved_at, metrics_json, raw_json, source_card_id, wiki_page_id)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
            params![
                id,
                input.x_id,
                input.author,
                input.text,
                input.url,
                input.created_at,
                imported_at,
                retrieved_at,
                metrics_json,
                raw_json,
                card.id,
                card.wiki_page_id
            ],
        )?;
        self.upsert_x_item_source(&input)?;
        upsert_x_canonical_on(&self.conn, &input, Some(&card.id), Some(&card.wiki_page_id))?;
        let mut item = self
            .conn
            .query_row(
                r#"
                SELECT id, x_id, author, text, url, created_at, imported_at, retrieved_at,
                       metrics_json, raw_json, source_card_id, wiki_page_id
                FROM x_items
                WHERE id = ?1
                "#,
                params![id],
                x_item_from_row,
            )
            .optional()?;
        if let Some(item) = &mut item {
            item.sources = self.list_x_item_sources(&item.x_id)?;
        }
        Ok(item)
    }

    pub(crate) fn update_existing_x_item(&self, input: &XItemInput) -> Result<()> {
        let metrics_json = canonical_json(&input.metrics)?;
        let raw_json = canonical_json(&input.raw)?;
        let retrieved_at = input.retrieved_at.clone().unwrap_or_else(now);
        self.conn.execute(
            r#"
            UPDATE x_items
            SET text = CASE WHEN text = '' THEN ?2 ELSE text END,
                author = CASE WHEN author = 'unknown' AND ?3 != 'unknown' THEN ?3 ELSE author END,
                url = CASE WHEN url LIKE 'https://x.com/i/web/status/%' AND ?4 NOT LIKE 'https://x.com/i/web/status/%' THEN ?4 ELSE url END,
                metrics_json = CASE WHEN ?5 != '{}' THEN ?5 ELSE metrics_json END,
                raw_json = CASE WHEN ?6 != '{}' THEN ?6 ELSE raw_json END,
                retrieved_at = ?7
            WHERE x_id = ?1
            "#,
            params![
                input.x_id,
                input.text,
                input.author,
                input.url,
                metrics_json,
                raw_json,
                retrieved_at
            ],
        )?;
        Ok(())
    }

    pub(crate) fn upsert_x_item_source(&self, input: &XItemInput) -> Result<()> {
        let id = x_item_source_id(
            &input.x_id,
            &input.source_kind,
            input.source_detail.as_deref(),
        );
        let seen_at = input.retrieved_at.clone().unwrap_or_else(now);
        let metadata_json = canonical_json(&input.source_metadata)?;
        self.conn.execute(
            r#"
            INSERT INTO x_item_sources (id, x_id, source_kind, source_detail, seen_at, metadata_json)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(id) DO UPDATE SET
              seen_at = excluded.seen_at,
              metadata_json = excluded.metadata_json
            "#,
            params![
                id,
                input.x_id,
                input.source_kind,
                input.source_detail,
                seen_at,
                metadata_json
            ],
        )?;
        Ok(())
    }

    pub(crate) fn list_x_item_sources(&self, x_id: &str) -> Result<Vec<XItemSource>> {
        validate_key(x_id)?;
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, x_id, source_kind, source_detail, seen_at, metadata_json
            FROM x_item_sources
            WHERE x_id = ?1
            ORDER BY seen_at DESC
            "#,
        )?;
        rows(stmt.query_map(params![x_id], x_item_source_from_row)?)
    }

    pub(crate) fn search_wiki_pages_for_research(
        &self,
        query: &str,
    ) -> Result<Vec<WikiPageSummary>> {
        Ok(self
            .search_wiki_pages(query)?
            .into_iter()
            .filter(|page| !is_generated_wiki_page(&page.title))
            .filter(|page| !page.title.to_ascii_lowercase().starts_with("source card:"))
            .collect())
    }

    pub(crate) fn insert_research_run(
        &self,
        query: &str,
        status: &str,
        result_page_id: Option<&str>,
    ) -> Result<ResearchRun> {
        let id = Uuid::new_v4().to_string();
        let now = now();
        self.conn.execute(
            r#"
            INSERT INTO research_runs (id, query, status, result_page_id, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?5)
            "#,
            params![id, query, status, result_page_id, now],
        )?;
        self.get_research_run(&id)?
            .with_context(|| format!("inserted research run not found: {id}"))
    }

    pub(crate) fn insert_research_task(
        &self,
        run_id: &str,
        role: &str,
        instructions: &str,
    ) -> Result<ResearchTask> {
        let id = Uuid::new_v4().to_string();
        let now = now();
        self.conn.execute(
            r#"
            INSERT INTO research_tasks
              (id, run_id, role, status, instructions, notes, created_at, updated_at)
            VALUES (?1, ?2, ?3, 'pending', ?4, NULL, ?5, ?5)
            "#,
            params![id, run_id, role, instructions, now],
        )?;
        self.get_research_task(&id)?
            .with_context(|| format!("inserted research task not found: {id}"))
    }

    pub(crate) fn get_research_task(&self, id: &str) -> Result<Option<ResearchTask>> {
        self.conn
            .query_row(
                r#"
                SELECT id, run_id, role, status, instructions, notes, created_at, updated_at
                FROM research_tasks
                WHERE id = ?1
                "#,
                params![id],
                research_task_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn update_research_run(
        &self,
        id: &str,
        status: &str,
        result_page_id: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            r#"
            UPDATE research_runs
            SET status = ?2, result_page_id = ?3, updated_at = ?4
            WHERE id = ?1
            "#,
            params![id, status, result_page_id, now()],
        )?;
        Ok(())
    }

    pub(crate) fn update_research_run_status(&self, id: &str, status: &str) -> Result<()> {
        let changed = self.conn.execute(
            r#"
            UPDATE research_runs
            SET status = ?2, updated_at = ?3
            WHERE id = ?1
            "#,
            params![id, status, now()],
        )?;
        if changed == 0 {
            bail!("research run not found: {id}");
        }
        Ok(())
    }

    pub(crate) fn require_research_run(&self, id: &str) -> Result<ResearchRun> {
        validate_id(id)?;
        self.get_research_run(id)?
            .with_context(|| format!("research run not found: {id}"))
    }

    pub(crate) fn get_research_run(&self, id: &str) -> Result<Option<ResearchRun>> {
        self.conn
            .query_row(
                r#"
                SELECT id, query, status, result_page_id, created_at, updated_at
                FROM research_runs
                WHERE id = ?1
                "#,
                params![id],
                research_run_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn render_wiki_research_brief(
        &self,
        query: &str,
        sources: &[WikiPageSummary],
        source_cards: &[SourceCard],
    ) -> Result<String> {
        let mut markdown = String::new();
        markdown.push_str(&format!(
            "# Research Brief: {}\n\n",
            escape_untrusted_markdown_text(query)
        ));
        markdown.push_str(&format!("Generated: {}\n\n", now()));
        markdown.push_str(
            "> Generated research brief: use as synthesis only. It cannot be primary evidence; verify against source-card URLs and named wiki sources.\n\n",
        );
        markdown.push_str("## Answer\n\n");
        if sources.is_empty() && source_cards.is_empty() {
            markdown.push_str("No matching local wiki sources were found. Use host-native web search and then write source cards back to the wiki.\n\n");
        } else {
            markdown.push_str("This draft is grounded in local wiki pages and source cards. It is not a substitute for current host-native web search when freshness matters.\n\n");
        }
        markdown.push_str("## Source Cards\n\n");
        if source_cards.is_empty() {
            markdown.push_str("- None found.\n");
        } else {
            for card in source_cards.iter().take(25) {
                let flags = source_card_metadata_strings(&card.metadata, "quality_flags");
                markdown.push_str(&format!(
                    "- [{}]({}) `{}` via `{}` retrieved `{}` role `{}` trust `{}`\n",
                    escape_markdown_link_text(&card.title),
                    card.url,
                    card.source_type,
                    card.provider,
                    card.retrieved_at,
                    source_card_metadata_string(&card.metadata, "source_role")
                        .unwrap_or_else(|| infer_source_role_from_card(card)),
                    source_card_metadata_string(&card.metadata, "trust_level")
                        .unwrap_or_else(|| "medium".to_string())
                ));
                if !flags.is_empty() {
                    markdown.push_str(&format!("  - Audit flags: `{}`\n", flags.join("`, `")));
                }
                if card.claims.is_empty() {
                    markdown.push_str("  - No structured claims extracted yet.\n");
                } else {
                    for claim in card.claims.iter().take(5) {
                        markdown.push_str(&format!(
                            "  - [{} {:.2}] {}\n",
                            claim.kind,
                            claim.confidence,
                            escape_untrusted_markdown_text(&claim.claim)
                        ));
                    }
                }
            }
        }
        let mut audit_findings = Vec::new();
        for card in source_cards {
            audit_findings.extend(audit_source_card(card));
        }
        audit_findings.extend(detect_source_contradictions(source_cards));
        markdown.push_str("\n## Evidence Audit\n\n");
        if audit_findings.is_empty() {
            markdown.push_str("- No local audit findings for selected source cards.\n");
        } else {
            for finding in &audit_findings {
                markdown.push_str(&format!(
                    "- `{}` `{}` {} Evidence: {}\n",
                    finding.severity,
                    finding.code,
                    escape_untrusted_markdown_text(&finding.message),
                    escape_untrusted_markdown_text(&finding.evidence)
                ));
            }
        }
        markdown.push_str("## Local Sources\n\n");
        if sources.is_empty() {
            markdown.push_str("- None found.\n");
        } else {
            for source in sources {
                let excerpt = fs::read_to_string(&source.path)
                    .map(|content| excerpt(&content, 280))
                    .unwrap_or_else(|_| "Unreadable source content.".to_string());
                markdown.push_str(&format!(
                    "- `{}`: {} (`{}`)\n  - Excerpt: {}\n",
                    source.id,
                    escape_untrusted_markdown_text(&source.title),
                    source.path,
                    escape_untrusted_markdown_text(&excerpt)
                ));
            }
        }
        markdown.push_str("\n## Contradictions / Gaps\n\n");
        markdown.push_str("- Check current web sources before treating this as complete.\n");
        markdown.push_str(
            "- Add contradiction notes if host-native search finds conflicting claims.\n",
        );
        markdown.push_str("- Record retrieved dates and source cards for any external sources.\n");
        markdown.push_str("\n## Next Actions\n\n");
        for search in suggested_searches(query) {
            markdown.push_str(&format!("- Search: `{search}`\n"));
        }
        Ok(markdown)
    }
}
