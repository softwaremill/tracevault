# TraceVault Compliance

TraceVault provides tamper-proof audit trails for AI-assisted software development. Every code change is cryptographically sealed and traceable to the AI session that produced it — giving regulated organizations the evidence they need to satisfy SOX, PCI-DSS, and SR 11-7 requirements.

## Why It Matters

When developers use AI coding assistants like Claude Code, the question every compliance officer asks is: **who wrote this code — the human or the AI?** And more importantly: **can you prove it?**

TraceVault answers both questions. It captures the full development session — every AI interaction, every file change, every tool call — and links that record to the resulting git commits with line-level attribution. Then it seals everything with cryptographic signatures and hash chains, making the record tamper-proof and independently verifiable.

## How It Works

### 1. Continuous Capture

TraceVault hooks into the developer's workflow at two points:

- **Session streaming** — As the developer works with an AI coding assistant, TraceVault captures the full session: prompts, responses, tool calls, file changes, and token usage. This happens in real time via lightweight hooks that don't interrupt the developer's flow.

- **Commit capture** — When the developer commits code, a post-commit hook sends the commit metadata and diff to TraceVault. The system automatically matches commit lines against recorded AI file changes to compute attribution: what percentage was AI-generated vs. human-written.

### 2. Cryptographic Sealing

When a commit arrives, TraceVault immediately seals it:

- **Record hash** — A SHA-256 hash of the commit data (author, message, diff, attribution) creates a unique fingerprint. Any change to the record — even a single character — produces a completely different hash.

- **Digital signature** — The record hash is signed with the organization's Ed25519 private key. This proves the seal was created by TraceVault and not forged.

- **Hash chain** — Each commit seal links to the previous one via a chain hash. This makes it impossible to insert, remove, or reorder records without breaking the chain. If someone deletes a commit from the middle of the history, every subsequent chain hash becomes invalid.

Session data is sealed alongside the commit, creating a linked pair: the code change and the AI activity that produced it.

### 3. Verification

TraceVault provides three verification mechanisms:

- **Dashboard verification** — One-click chain integrity check from the compliance dashboard. Walks the entire hash chain, verifies every signature, and reports pass/fail with detailed error information.

- **CI/CD verification** — API endpoint for build pipelines. Before deploying code, CI can verify that every commit in the release is registered, sealed, signed, and policy-compliant. Failed verification can block deployment.

- **Independent verification** — The organization's public key is available via API. Auditors can independently verify signatures without needing access to TraceVault's signing key.

## Compliance Modes

TraceVault supports pre-configured compliance modes that enforce appropriate controls:

| Mode | Retention | Use Case |
|------|-----------|----------|
| **SOX** | 7 years | Public companies, financial reporting systems |
| **PCI-DSS** | 1 year | Payment processing, cardholder data systems |
| **SR 11-7** | 3 years | Banking, model risk management |
| **Custom** | Configurable | Organization-specific requirements |

Each mode sets minimum data retention periods and can enforce mandatory signing and verification intervals.

## What Gets Sealed

Every seal captures a complete, verifiable record:

**Commit seal:**
- Git SHA, branch, author, commit message
- Full structured diff (files, hunks, line changes)
- Timestamp
- AI attribution summary (percentage of lines attributed to AI sessions)

**Session seal (linked to commit):**
- Session identifier and repository
- All AI tool calls with content hashes
- All file changes with content hashes
- Token usage and cost
- Timestamp of seal

## Key Properties

**Tamper-evident** — Any modification to sealed data breaks the hash chain. Chain verification detects tampering immediately, pinpointing exactly which record was altered.

**Non-repudiable** — Digital signatures prove that seals were created by the organization's TraceVault instance. Signatures cannot be forged without the private key.

**Independently verifiable** — The public key is available for external auditors. Verification does not require trust in TraceVault's infrastructure.

**Key rotation support** — Signing keys can be rotated. Historical seals remain verifiable because TraceVault records which key was active at each point in time.

**Complete provenance** — Every line of code can be traced: commit to session to individual AI interaction. Auditors can see not just *what* changed, but *how* it was produced.

## Audit Log

All compliance-relevant actions are recorded in an immutable audit log, filterable by action type, actor, resource, and date range.

**Organization management:**
- `org.create` — Organization created
- `llm_settings.update` — LLM provider/model settings changed

**User and role management:**
- `user.register` — New user registered (with org creation)
- `role.change` — Member role changed
- `member.remove` — Member removed from organization
- `invite.create` — Invite sent to email
- `invite.revoke` — Pending invite revoked
- `invite.accept` — User joined organization via invite (new or existing account)
- `invitation_request.approve` — Admin approved a join request
- `invitation_request.reject` — Admin rejected a join request

**Compliance and sealing:**
- `org.compliance.update` — Compliance mode or retention changed
- `chain.verify` — Hash chain verification run
- `ci.verify` — CI commit verification run
- `commit.sealed` — Commit cryptographically sealed

**Policies:**
- `policy.create` — Policy created
- `policy.update` — Policy updated
- `policy.delete` — Policy deleted
- `policy.check` — Policy check executed

**Model pricing:**
- `create` / `update` / `pricing_sync` — Model pricing configuration changes

## Integration

TraceVault compliance integrates into existing workflows:

- **Git hooks** — Automatic, zero-friction commit capture
- **CI/CD APIs** — Verification gates in deployment pipelines
- **Dashboard** — Real-time compliance status for compliance officers
- **Public key API** — External auditor access for independent verification
