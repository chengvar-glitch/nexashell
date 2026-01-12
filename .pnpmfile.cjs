function readPackage(pkg, context) {
  // 确保项目只使用pnpm
  if (pkg.name === 'nexashell') {
    pkg.pnpm = pkg.pnpm || {};
    pkg.pnpm.onlyBuiltDependencies = pkg.pnpm.onlyBuiltDependencies || [];
  }
  return pkg;
}

module.exports = {
  hooks: {
    readPackage
  }
};