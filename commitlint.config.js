module.exports = {
  extends: ['@commitlint/config-conventional'],
  parserPreset: {
    parserOpts: {
      headerPattern: /^(?:\[(\d+)\] )?([\w]+)(?:\(([\w\$\.\-\*\s]*)\))?\: (.+)$/,
      headerCorrespondence: ['number', 'type', 'scope', 'subject']
    }
  },
  rules: {
    'type-enum': [
      2,
      'always',
      [
        'feat',     // New feature
        'fix',      // Bug fix
        'docs',     // Documentation changes
        'style',    // Code style changes (formatting, etc)
        'refactor', // Code refactoring
        'perf',     // Performance improvements
        'test',     // Adding or updating tests
        'build',    // Changes to build system or dependencies
        'ci',       // CI/CD changes
        'chore',    // Other changes that don't modify src or test files
        'revert'    // Revert a previous commit
      ]
    ],
    'subject-case': [2, 'never', ['sentence-case', 'start-case', 'pascal-case', 'upper-case']],
    'subject-empty': [2, 'never'],
    'subject-full-stop': [2, 'never', '.'],
    'type-case': [2, 'always', 'lower-case'],
    'type-empty': [2, 'never'],
    'header-max-length': [2, 'always', 150],
    'body-max-line-length': [2, 'always', 200]
  }
};
