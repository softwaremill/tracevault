#!/usr/bin/env python3
# /// script
# requires-python = ">=3.10"
# dependencies = [
#     "psycopg2-binary",
#     "pynacl",
# ]
# ///
"""
Re-seal the entire TraceVault chain with a new Ed25519 signing key.

Generates a fresh key, recomputes record_hash / chain_hash / signature
for every commit (per-repo, chronological) and every session, then
prints the base64 key so you can set TRACEVAULT_SIGNING_KEY.

Usage:
    export DATABASE_URL=postgres://tracevault:tracevault@localhost:5432/tracevault
    uv run scripts/reseal-chain.py
"""

import hashlib
import json
import os
import sys
from base64 import b64encode
try:
    import psycopg2
    import psycopg2.extras
except ImportError:
    sys.exit("Missing dependency: pip install psycopg2-binary")

try:
    from nacl.signing import SigningKey
except ImportError:
    sys.exit("Missing dependency: pip install pynacl")


def record_hash(canonical_json_bytes: bytes) -> str:
    return hashlib.sha256(canonical_json_bytes).hexdigest()


def chain_hash(prev_chain_hash: str | None, rec_hash: str) -> str:
    h = hashlib.sha256()
    if prev_chain_hash:
        h.update(prev_chain_hash.encode())
    h.update(rec_hash.encode())
    return h.hexdigest()


def sign(signing_key: SigningKey, rec_hash: str) -> str:
    signed = signing_key.sign(rec_hash.encode())
    return b64encode(signed.signature).decode()


def main():
    db_url = os.environ.get("DATABASE_URL")
    if not db_url:
        sys.exit("Set DATABASE_URL environment variable")

    # Generate new key
    signing_key = SigningKey.generate()
    seed_b64 = b64encode(bytes(signing_key)).decode()
    public_b64 = b64encode(bytes(signing_key.verify_key)).decode()

    conn = psycopg2.connect(db_url)
    conn.autocommit = False
    cur = conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor)

    # --- Re-seal commits per repo, in chronological order ---
    cur.execute("SELECT DISTINCT repo_id FROM commits ORDER BY repo_id")
    repo_ids = [row["repo_id"] for row in cur.fetchall()]

    total_commits = 0
    for repo_id in repo_ids:
        cur.execute(
            """SELECT id, commit_sha, branch, author, repo_id, created_at
               FROM commits
               WHERE repo_id = %s
               ORDER BY created_at ASC, id ASC""",
            (str(repo_id),),
        )
        commits = cur.fetchall()

        prev = None
        for c in commits:
            canonical = json.dumps(
                {
                    "commit_sha": c["commit_sha"],
                    "branch": c["branch"],
                    "author": c["author"],
                    "repo_id": str(c["repo_id"]),
                },
                sort_keys=True,
                separators=(",", ":"),
            ).encode()

            rh = record_hash(canonical)
            ch = chain_hash(prev, rh)
            sig = sign(signing_key, rh)

            # Use created_at as sealed_at so ordering is deterministic
            cur.execute(
                """UPDATE commits
                   SET record_hash = %s,
                       chain_hash = %s,
                       prev_chain_hash = %s,
                       signature = %s,
                       sealed_at = %s
                   WHERE id = %s""",
                (rh, ch, prev, sig, c["created_at"], str(c["id"])),
            )

            prev = ch
            total_commits += 1

    # --- Re-seal sessions ---
    cur.execute(
        """SELECT s.id, s.session_id, s.commit_id, s.model, s.tool, s.created_at
           FROM sessions s
           ORDER BY s.created_at ASC, s.id ASC"""
    )
    sessions = cur.fetchall()

    total_sessions = 0
    for s in sessions:
        canonical = json.dumps(
            {
                "session_id": s["session_id"],
                "commit_id": str(s["commit_id"]),
                "model": s["model"],
                "tool": s["tool"],
            },
            sort_keys=True,
            separators=(",", ":"),
        ).encode()

        rh = record_hash(canonical)
        sig = sign(signing_key, rh)

        cur.execute(
            """UPDATE sessions
               SET record_hash = %s,
                   signature = %s,
                   sealed_at = %s
               WHERE id = %s""",
            (rh, sig, s["created_at"], str(s["id"])),
        )
        total_sessions += 1

    conn.commit()
    cur.close()
    conn.close()

    print(f"Re-sealed {total_commits} commits across {len(repo_ids)} repos")
    print(f"Re-sealed {total_sessions} sessions")
    print()
    print(f"Public key (base64): {public_b64}")
    print()
    print("Set this environment variable before starting the server:")
    print(f"  export TRACEVAULT_SIGNING_KEY={seed_b64}")


if __name__ == "__main__":
    main()
