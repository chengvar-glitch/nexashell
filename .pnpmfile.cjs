function readPackage(pkg, context) {
  if (pkg.name === 'nexashell') {
    pkg.pnpm = pkg.pnpm || {};
    pkg.pnpm.onlyBuiltDependencies = pkg.pnpm.onlyBuiltDependencies || [];
  }
  return pkg;
}

module.exports = {
  hooks: {
    readPackage,
  },
};
