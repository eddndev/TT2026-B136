#!/usr/bin/env bash
#
# flip-byte.sh -- flip exactly one byte of a file in place.
#
# Usage: flip-byte.sh FILE [OFFSET]
#
#   FILE    regular file to modify in place
#   OFFSET  zero-based byte offset of the byte to flip (default: 0)
#
# The byte at OFFSET is XOR-ed with 0xff (its bitwise complement), so
# running the script twice with the same offset restores the original
# file. The file length never changes. This is used to demonstrate
# tamper detection: any integrity check over the file must fail after a
# single byte has been flipped.

set -euo pipefail

usage() {
  echo "usage: $(basename "$0") FILE [OFFSET]" >&2
  echo "  FILE    regular file to modify in place" >&2
  echo "  OFFSET  zero-based byte offset to flip (default: 0)" >&2
}

if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
  usage
  exit 1
fi

file="$1"
offset="${2-0}"

if [ ! -e "$file" ]; then
  echo "error: file not found: $file" >&2
  exit 1
fi
if [ ! -f "$file" ]; then
  echo "error: not a regular file: $file" >&2
  exit 1
fi
if [ ! -r "$file" ] || [ ! -w "$file" ]; then
  echo "error: file must be readable and writable: $file" >&2
  exit 1
fi

case "$offset" in
  '' | *[!0-9]*)
    echo "error: OFFSET must be a non-negative integer, got: $offset" >&2
    exit 1
    ;;
esac

size=$(wc -c < "$file" | tr -d '[:space:]')

if [ "$size" -eq 0 ]; then
  echo "error: file is empty, there is no byte to flip: $file" >&2
  exit 1
fi
if [ "$offset" -ge "$size" ]; then
  echo "error: OFFSET $offset is out of range (file size is $size bytes)" >&2
  exit 1
fi

# Read the current byte at OFFSET as a decimal value.
old=$(dd if="$file" bs=1 skip="$offset" count=1 2> /dev/null \
  | od -An -tu1 | tr -d '[:space:]')

new=$((old ^ 0xff))

# Write the flipped byte back in place. conv=notrunc keeps the rest of
# the file (and its length) untouched.
printf '%b' "\\0$(printf '%03o' "$new")" \
  | dd of="$file" bs=1 seek="$offset" count=1 conv=notrunc 2> /dev/null

printf 'flipped byte at offset %s of %s: 0x%02x -> 0x%02x (size %s bytes, unchanged)\n' \
  "$offset" "$file" "$old" "$new" "$size"
