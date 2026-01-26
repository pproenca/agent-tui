const path = require('path');

const PLATFORM_MAP = {
  darwin: 'darwin',
  linux: 'linux',
};

const ARCH_MAP = {
  x64: 'x64',
  arm64: 'arm64',
};

function getPlatformArch() {
  const platform = PLATFORM_MAP[process.platform];
  const arch = ARCH_MAP[process.arch];

  if (!platform || !arch) {
    return null;
  }

  return `${platform}-${arch}`;
}

function getPackageName(platformArch) {
  return `agent-tui-${platformArch}`;
}

function getBinaryName() {
  return process.platform === 'win32' ? 'agent-tui.exe' : 'agent-tui';
}

function resolveBinaryPath() {
  const platformArch = getPlatformArch();
  if (!platformArch) {
    return { platformArch: null, pkgName: null, binPath: null };
  }

  const pkgName = getPackageName(platformArch);
  const root = path.resolve(__dirname, '..');

  try {
    const pkgJsonPath = require.resolve(`${pkgName}/package.json`, { paths: [root] });
    const pkgRoot = path.dirname(pkgJsonPath);
    const binPath = path.join(pkgRoot, 'bin', getBinaryName());
    return { platformArch, pkgName, binPath };
  } catch (error) {
    return { platformArch, pkgName, binPath: null, error };
  }
}

module.exports = {
  getPlatformArch,
  getPackageName,
  getBinaryName,
  resolveBinaryPath,
};
