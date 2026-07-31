-- Public cross-household report links (Phase 6, PUBREP-01..07)
--
-- A user configures named reports in their user settings. Each report spans an explicitly
-- chosen set of the households they belong to (D-01) and is retrievable without
-- authentication through an unguessable UUID token (D-05).
CREATE TABLE public_reports (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    -- D-05: UUID v4, regenerable. UNIQUE so a regenerated token can never collide.
    token TEXT NOT NULL UNIQUE,
    -- D-06: output language per report ('en' or 'de'), validated in the service layer.
    language TEXT NOT NULL DEFAULT 'en',
    -- D-05: a disabled report answers 404 on its URL without losing its configuration.
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_public_reports_user ON public_reports(user_id);
-- No separate token index: the UNIQUE constraint above already provides one, and the
-- public endpoint's per-request lookup by token uses it.

-- D-01: the explicit household selection.
--
-- The REFERENCES clauses document intent but do NOT enforce anything at runtime: this
-- project never enables `PRAGMA foreign_keys`, matching every table that came before.
-- The service layer therefore deletes junction rows explicitly, and a row pointing at a
-- deleted household is harmless — the D-08 membership check drops it from the output.
CREATE TABLE public_report_households (
    report_id TEXT NOT NULL REFERENCES public_reports(id) ON DELETE CASCADE,
    household_id TEXT NOT NULL REFERENCES households(id) ON DELETE CASCADE,
    PRIMARY KEY (report_id, household_id)
);

CREATE INDEX idx_public_report_households_report ON public_report_households(report_id);
