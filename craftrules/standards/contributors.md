# Contributor credits (About window)

Every app credits its contributors in **About ▸ Contributors**, and the AI models named in
`Co-Authored-By` trailers in **About ▸ Models**. Two files, both **compiled into the binary**
by the UI crate's `build.rs` — nothing is read from disk or the network at run time:

- `contributors/contributors.json` — commit stats per GitHub username (schema below).
  **Generated** by `scripts/contributors.py` in this repo; never hand-edit.
- `contributors/people.toml` — the names people chose for themselves. Each contributor adds
  **only their own** entry, in their own PR.

## Rules

- Agents preparing a PR check whether their human's GitHub username has a
  `[people.<username>]` entry. If not, ask **once** whether they want more than the username:
  a real name, a display name, and/or their public GitHub profile name
  (`sync_github_name = true`). If yes, add only their entry; if no, change nothing — they are
  credited as `@username` anyway.
- **Never** add, edit, guess or copy anyone else's entry or name (not from git config,
  commit authors or GitHub profiles). A maintainer may add a person only when that person
  asked, recording where in `consent_source`.
- Maintainers refresh stats with `python3 craftrules/scripts/contributors.py .`
  (re-verifies who wrote each `people.toml` entry); `--check` only verifies.

## people.toml

```toml
[people.your-github-username]
real_name = "Your Name"        # shown when the viewer picks "Real name"
display_name = "Nickname"      # shown when the viewer picks "Display name"
sync_github_name = true        # use the public GitHub profile name as display name
```

Every field is optional. To remove your names, delete your entry.

## contributors.json (schema 1)

```json
{"schema": 1, "repo": "<org>/<repo>", "branch": "main", "head": "<sha>",
 "head_date": "<iso>", "commits": N, "unattributed_commits": N,
 "contributors": [{"login": "…", "people_entry": true, "github_name": null,
   "prs": 0, "commits": N, "lines_added": N, "lines_deleted": N,
   "binary_added": N, "binary_deleted": N,
   "first_commit": "<iso>", "last_commit": "<iso>"}],
 "models": [{"name": "…", "commits": N}]}
```

`models` is aggregated from `Co-Authored-By:` trailers.
