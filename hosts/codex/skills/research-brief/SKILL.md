# Research Brief

Use this skill only when turning already-collected source material into a report draft or executive summary artifact.

This is not a quick research mode. If the user asks to research a topic, use
`deep-research` instead. Use this skill when the source set already exists and
the task is to render or audit a concise artifact from that evidence.

Rules:

- Search the wiki before drafting.
- Use `research_brief_from_wiki` for the first local report/summary draft.
- Use `research_audit` when the artifact may influence decisions, publication, or project state.
- Read the cited wiki pages and check that the draft did not overstate them.
- Add a short contradiction/gaps section when sources disagree or freshness is uncertain.
- Do not cite generated `Research Brief:` pages as primary sources.
- If the artifact is for publishing, also apply the user's style and voice guidance before final prose.

Typical commands:

```sh
arcwell wiki search <query>
arcwell research brief <query> --no-write
arcwell research audit <query>
arcwell research brief <query>
arcwell research runs
```

MCP tools:

- `wiki_search`
- `wiki_read`
- `research_brief_from_wiki`
- `research_audit`
- `research_runs`
- `research_tasks`
