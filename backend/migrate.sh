#!/usr/bin/env bash
cd "$(dirname "$0")"
export DATABASE_URL="sqlite://../household.db?mode=rwc"
sqlx "$@"
