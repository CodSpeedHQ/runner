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

# Locally, simply search and display
if [ "$CI" != "true" ]; then
    echo "Searching todos for $LINEAR_ISSUE"
    grep "TODO($LINEAR_ISSUE)" . -Rni --color --exclude-dir=".git"

    exit 0
fi

# On the CI, annotate files
# https://docs.github.com/en/actions/writing-workflows/choosing-what-your-workflow-does/workflow-commands-for-github-actions#setting-a-notice-message
grep "TODO($LINEAR_ISSUE)" . -Rni --exclude-dir=".git" | awk -F":" {'print "::notice file="$1",line="$2",title=TODO::"$4'}
