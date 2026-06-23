#!/usr/bin/env bash
# Provision Cloudflare resources and deploy the kontu Worker.
# Requires: `wrangler login` (or CLOUDFLARE_API_TOKEN exported) beforehand.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/worker"

if ! npx wrangler whoami >/dev/null 2>&1; then
  echo "Not logged in to Cloudflare. Run 'wrangler login' or export CLOUDFLARE_API_TOKEN, then re-run." >&2
  exit 1
fi

echo "==> Provisioning D1 database 'kontu'"
npx wrangler d1 create kontu 2>/dev/null || echo "    (already exists)"
DB_ID=$(npx wrangler d1 list --json 2>/dev/null \
  | node -e "let d='';process.stdin.on('data',c=>d+=c).on('end',()=>{const a=JSON.parse(d);const m=a.find(x=>x.name==='kontu');process.stdout.write(m?(m.uuid||m.database_id||''):'')})")
[ -n "$DB_ID" ] || { echo "Could not determine the D1 database id" >&2; exit 1; }
echo "    D1 id: $DB_ID"

echo "==> Writing database_id into wrangler.jsonc"
node -e "const fs=require('fs');const p='wrangler.jsonc';let s=fs.readFileSync(p,'utf8');s=s.replace(/(\"database_id\":\s*\")[^\"]*(\")/, '\$1$DB_ID\$2');fs.writeFileSync(p,s)"

echo "==> Provisioning R2 bucket 'kontu-photos'"
npx wrangler r2 bucket create kontu-photos 2>/dev/null || echo "    (already exists)"

echo "==> Applying migrations + seed to the remote D1"
npx wrangler d1 migrations apply kontu --remote
npx wrangler d1 execute kontu --remote --file=seed.sql

echo "==> Set the API_TOKEN secret (this is the bearer token the TUI sends)"
npx wrangler secret put API_TOKEN

echo "==> Deploying"
npx wrangler deploy

echo
echo "Done. Put the deployed https URL and the API_TOKEN you chose into:"
echo "  ~/.config/kontu/config.toml"
