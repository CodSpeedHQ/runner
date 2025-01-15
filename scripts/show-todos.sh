if [ "$CI" = "true" ]; then
    BRANCH_NAME=$GITHUB_HEAD_REF
else
    BRANCH_NAME=$(git rev-parse --abbrev-ref HEAD)
fi

LINEAR_ISSUE=$(echo "$BRANCH_NAME" | grep -oP '^cod-(\d+)')

if [ "$LINEAR_ISSUE" = "" ]; then
    echo "No Linear Issue found, skipping"

    exit 0
fi

echo "Searching todos for $LINEAR_ISSUE"

grep "TODO($LINEAR_ISSUE)" . -Rni --color --exclude-dir=".git"
