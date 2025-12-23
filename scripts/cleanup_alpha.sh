#!/bin/bash
set -e

echo "=== Alpha Release Cleanup Script ==="
echo ""

# Fetch all alpha releases
echo "Fetching alpha releases..."
RELEASES=$(gh release list --limit 1000 | grep "alpha" | awk -F'\t' '{print $3}' || true)

# Fetch all alpha tags
echo "Fetching alpha tags..."
git fetch --tags 2>/dev/null || true
TAGS=$(git tag -l "*alpha*" || true)

# Combine and get unique names
ALL_ITEMS=$(echo -e "$RELEASES\n$TAGS" | sort -u | grep -v '^$' || true)

echo ""
echo "========================================="
echo "FOUND ITEMS:"
echo "========================================="
echo ""

if [ -z "$ALL_ITEMS" ]; then
    echo "No alpha releases or tags found."
    exit 0
fi

# Display each item with its status
for ITEM in $ALL_ITEMS; do
    HAS_RELEASE=""
    HAS_TAG=""

    if echo "$RELEASES" | grep -q "^${ITEM}$"; then
        HAS_RELEASE="✓"
    else
        HAS_RELEASE="✗"
    fi

    if echo "$TAGS" | grep -q "^${ITEM}$"; then
        HAS_TAG="✓"
    else
        HAS_TAG="✗"
    fi

    echo "  - $ITEM [Release: $HAS_RELEASE | Tag: $HAS_TAG]"
done

echo ""
echo "========================================="
echo ""

# Process each item
for ITEM in $ALL_ITEMS; do
    HAS_RELEASE=false
    HAS_TAG=false

    if echo "$RELEASES" | grep -q "^${ITEM}$"; then
        HAS_RELEASE=true
    fi

    if echo "$TAGS" | grep -q "^${ITEM}$"; then
        HAS_TAG=true
    fi

    read -p "Delete '$ITEM'? (y/N) " -n 1 -r
    echo ""

    if [[ $REPLY =~ ^[Yy]$ ]]; then
        # Delete release if it exists
        if [ "$HAS_RELEASE" = true ]; then
            echo -n "  Deleting release... "
            if gh release delete "$ITEM" --yes 2>/dev/null; then
                echo "✓ deleted"
            else
                echo "✗ failed"
            fi
        fi

        # Delete remote tag if it exists
        if [ "$HAS_TAG" = true ]; then
            echo -n "  Deleting remote tag... "
            if git push origin ":refs/tags/$ITEM" 2>/dev/null; then
                echo "✓ deleted"
            else
                echo "✗ failed (may not exist on remote)"
            fi

            # Delete local tag
            echo -n "  Deleting local tag... "
            if git tag -d "$ITEM" 2>/dev/null; then
                echo "✓ deleted"
            else
                echo "✗ failed"
            fi
        fi
    else
        echo "  Skipped."
    fi
    echo ""
done

echo "========================================="
echo "✓ Cleanup complete!"
echo "========================================="
