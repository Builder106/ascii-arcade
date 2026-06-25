#!/bin/zsh
set -euo pipefail

# One-time: create a stable self-signed code-signing identity so rebuilds keep
# their TCC grants (Accessibility, Screen Recording, etc.). Ad-hoc signing
# (`codesign --sign -`) pins the designated requirement to the binary's cdhash,
# which changes every build — so macOS re-prompts for permissions each time.
# A self-signed identity makes the requirement identity-based and stable.
#
# Re-running is safe: it no-ops if the identity already exists.

IDENTITY_NAME="${SIGN_IDENTITY:-ASCII Arcade Local}"
KEYCHAIN="$HOME/Library/Keychains/login.keychain-db"

if security find-identity -v -p codesigning | grep -qF "$IDENTITY_NAME"; then
	echo "Signing identity '$IDENTITY_NAME' already exists — nothing to do."
	exit 0
fi

echo "Creating self-signed code-signing identity '$IDENTITY_NAME'…"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cat > "$TMP/openssl.cnf" <<CNF
[ req ]
distinguished_name = dn
x509_extensions    = v3
prompt             = no
[ dn ]
CN = $IDENTITY_NAME
[ v3 ]
basicConstraints     = critical,CA:false
keyUsage             = critical,digitalSignature
extendedKeyUsage     = critical,codeSigning
CNF

openssl req -x509 -newkey rsa:2048 -nodes \
	-keyout "$TMP/key.pem" -out "$TMP/cert.pem" \
	-days 3650 -config "$TMP/openssl.cnf" >/dev/null 2>&1

# Use legacy PKCS#12 algorithms + a throwaway password: macOS's `security
# import` can't verify the SHA-256 MAC that OpenSSL 3 writes by default, and
# rejects empty-password bundles with "MAC verification failed".
P12_PASS="ascii-arcade"
openssl pkcs12 -export -out "$TMP/id.p12" \
	-inkey "$TMP/key.pem" -in "$TMP/cert.pem" \
	-name "$IDENTITY_NAME" -passout "pass:$P12_PASS" \
	-legacy -keypbe PBE-SHA1-3DES -certpbe PBE-SHA1-3DES -macalg sha1 >/dev/null 2>&1

# Import the key+cert; -T /usr/bin/codesign whitelists codesign on the key ACL.
security import "$TMP/id.p12" -k "$KEYCHAIN" -P "$P12_PASS" -T /usr/bin/codesign

# Let codesign use the key non-interactively (no "codesign wants to sign" popup
# each build). This needs your login-keychain password. If you skip it, signing
# still works — you'll just get a one-time codesign keychain prompt where you
# click "Always Allow". Non-fatal either way.
echo "Authorizing codesign to use the key without prompting each build."
printf "Enter your login-keychain password (or press Return to skip): "
read -rs KC_PASS; echo
if [ -n "$KC_PASS" ]; then
	if security set-key-partition-list -S apple-tool:,apple: -s -k "$KC_PASS" "$KEYCHAIN" >/dev/null 2>&1; then
		echo "codesign authorized."
	else
		echo "warning: could not set partition list (wrong password?). You'll get a" >&2
		echo "         one-time codesign prompt on first build — click 'Always Allow'." >&2
	fi
else
	echo "Skipped. On the next build, click 'Always Allow' if codesign prompts."
fi

echo "Done. Identity '$IDENTITY_NAME' is ready."
echo "Now run ./scripts/reinstall.sh; grant Accessibility once more and it will stick."
