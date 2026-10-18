#!/usr/bin/env python3
"""Regenerate contributors/contributors.json and manage contributors/people.toml.

Usage:
  contributors.py REPO                 regenerate contributors.json from git history
  contributors.py --check REPO         verify only: exit 1 if contributors.json is stale
  contributors.py --add-me REPO [--real-name "…"] [--display-name "…"] [--sync-github-name]
                                       append YOUR OWN entry to people.toml

Standard: standards/contributors.md. stdlib only; `gh` is used when present for GitHub
logins / PR counts / profile names, and skipped silently when it is not.
"""

import json
import re
import subprocess
import sys
import urllib.request
from collections import defaultdict
from pathlib import Path

SCHEMA = 1


def git(repo, *args):
    try:
        return subprocess.run(
            ["git", "-C", repo, *args], capture_output=True, text=True, check=True
        ).stdout
    except subprocess.CalledProcessError as e:
        sys.exit(f"git {' '.join(args)} failed in {repo}: {e.stderr.strip()}")


def gh_json(path):
    """Query the GitHub API via gh, or return None when gh is unavailable/fails."""
    try:
        out = subprocess.run(
            ["gh", "api", path], capture_output=True, text=True, check=True
        ).stdout
        return json.loads(out)
    except (OSError, subprocess.CalledProcessError, json.JSONDecodeError):
        return None


def load_people(path):
    """Parse people.toml without a TOML lib: {username: {field: value}}."""
    people = {}
    if not path.is_file():
        return people
    current = None
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.split("#", 1)[0].strip()
        m = re.fullmatch(r"\[people\.([^\]]+)\]", line)
        if m:
            current = m.group(1)
            people[current] = {}
            continue
        m = re.fullmatch(r'(real_name|display_name)\s*=\s*"([^"]*)"', line)
        if m and current is not None:
            people[current][m.group(1)] = m.group(2)
            continue
        m = re.fullmatch(r"(sync_github_name|consent_source)\s*=\s*(true|false|\"[^\"]*\")", line)
        if m and current is not None:
            people[current][m.group(1)] = m.group(2).strip('"')
    return people


def login_for(repo, name, email):
    """Best-effort GitHub login for a commit author."""
    gh = gh_json(f"search/users?q={urllib.request.quote(email)}+in:email")
    if gh and gh.get("items"):
        return gh["items"][0]["login"]
    return email.split("@", 1)[0] if email else name


def collect_stats(repo):
    """Walk git history once; return (per-author stats, per-model commit counts)."""
    stats = defaultdict(
        lambda: {
            "commits": 0,
            "lines_added": 0,
            "lines_deleted": 0,
            "binary_added": 0,
            "binary_deleted": 0,
            "first_commit": None,
            "last_commit": None,
        }
    )
    authors = {}
    models = defaultdict(int)
    # %x1f is the field separator; a record is a line starting with the sentinel
    out = git(
        repo,
        "log",
        "--numstat",
        "--format=COMMIT%x1f%aN%x1f%aE%x1f%aI",
        "--all",
    )
    key = None
    for line in out.splitlines():
        if line.startswith("COMMIT\x1f"):
            _, name, email, date = line.split("\x1f")
            authors[email] = name
            key = email
            s = stats[key]
            s["commits"] += 1
            s["last_commit"] = s["last_commit"] or date
            s["first_commit"] = date
            continue
        if key is None:
            continue
        parts = line.split("\t")
        if len(parts) == 3:
            added, deleted = parts[0], parts[1]
            if added == "-" or deleted == "-":
                stats[key]["binary_added"] += 1 if added != "-" else 0
                stats[key]["binary_deleted"] += 1 if deleted != "-" else 0
            else:
                stats[key]["lines_added"] += int(added or 0)
                stats[key]["lines_deleted"] += int(deleted or 0)
    bodies = git(repo, "log", "--format=%(trailers:key=Co-Authored-By,valueonly)", "--all")
    for line in bodies.splitlines():
        line = line.strip()
        if line:
            name = re.sub(r"\s*<[^>]*>\s*$", "", line).strip()
            if name:
                models[name] += 1
    return stats, authors, models


def regenerate(repo):
    repo_path = Path(repo)
    people = load_people(repo_path / "contributors" / "people.toml")
    stats, authors, models = collect_stats(repo)
    remote = ""
    try:
        remote = git(repo, "remote", "get-url", "origin").strip()
    except subprocess.CalledProcessError:
        pass
    repo_slug = re.sub(r"\.git$", "", remote.split(":")[-1]) if remote else repo_path.name
    branch = git(repo, "rev-parse", "--abbrev-ref", "HEAD").strip()
    contributors = []
    for email, s in sorted(stats.items(), key=lambda kv: kv[0].lower()):
        login = login_for(repo, authors[email], email)
        github_name = None
        if people.get(login, {}).get("sync_github_name") == "true":
            user = gh_json(f"users/{urllib.request.quote(login)}")
            if user:
                github_name = user.get("name")
        contributors.append(
            {
                "login": login,
                "people_entry": login in people,
                "github_name": github_name,
                "prs": 0,
                "commits": s["commits"],
                "lines_added": s["lines_added"],
                "lines_deleted": s["lines_deleted"],
                "binary_added": s["binary_added"],
                "binary_deleted": s["binary_deleted"],
                "first_commit": s["first_commit"],
                "last_commit": s["last_commit"],
            }
        )
    head = git(repo, "rev-parse", "HEAD").strip()
    head_date = git(repo, "show", "-s", "--format=%aI", "HEAD").strip()
    return {
        "schema": SCHEMA,
        "repo": repo_slug,
        "branch": branch,
        "head": head,
        "head_date": head_date,
        "commits": sum(s["commits"] for s in stats.values()),
        "unattributed_commits": 0,
        "contributors": contributors,
        "models": [
            {"name": n, "commits": c} for n, c in sorted(models.items())
        ],
    }


def add_me(repo, real_name, display_name, sync_github_name):
    user = gh_json("user")
    if user and user.get("login"):
        login = user["login"]
    else:
        login = git(repo, "config", "user.name").strip() or git(
            repo, "config", "user.email"
        ).strip().split("@", 1)[0]
        if not login:
            sys.exit("cannot determine your username (set git user.name or install gh)")
    path = Path(repo) / "contributors" / "people.toml"
    people = load_people(path)
    if login in people:
        sys.exit(f"[people.{login}] already exists — edit only your own entry")
    block = f"\n[people.{login}]\n"
    if real_name:
        block += f'real_name = "{real_name}"\n'
    if display_name:
        block += f'display_name = "{display_name}"\n'
    block += f"sync_github_name = {'true' if sync_github_name else 'false'}\n"
    with path.open("a", encoding="utf-8") as f:
        f.write(block)
    print(f"added [people.{login}] to {path}")


def main(argv):
    args = [a for a in argv if not a.startswith("--")]
    check = "--check" in argv
    add = "--add-me" in argv
    if not args:
        sys.exit(__doc__)
    repo = args[0]
    if add:
        def opt(flag):
            return argv[argv.index(flag) + 1] if flag in argv else None

        add_me(
            repo,
            opt("--real-name"),
            opt("--display-name"),
            "--sync-github-name" in argv,
        )
        return
    data = regenerate(repo)
    out = Path(repo) / "contributors" / "contributors.json"
    rendered = json.dumps(data, indent=2, ensure_ascii=False) + "\n"
    if check:
        if out.is_file() and out.read_text(encoding="utf-8") == rendered:
            print("contributors.json is up to date")
            return
        sys.exit("contributors.json is stale — run contributors.py to regenerate")
    out.parent.mkdir(exist_ok=True)
    out.write_text(rendered, encoding="utf-8")
    print(f"wrote {out} ({len(data['contributors'])} contributors)")


if __name__ == "__main__":
    main(sys.argv[1:])
