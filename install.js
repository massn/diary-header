#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const https = require('https');
const { execSync } = require('child_process');

const REPO = 'massn/diary-header';
const VERSION = require('./package.json').version;

function getPlatform() {
  const platform = process.platform;
  const arch = process.arch;

  if (platform === 'darwin') {
    if (arch === 'arm64') return 'aarch64-apple-darwin';
    return 'x86_64-apple-darwin';
  }
  if (platform === 'linux') {
    if (arch === 'arm64') return 'aarch64-unknown-linux-gnu';
    return 'x86_64-unknown-linux-gnu';
  }
  if (platform === 'win32') {
    return 'x86_64-pc-windows-msvc';
  }

  throw new Error(`Unsupported platform: ${platform} ${arch}`);
}

function getDownloadUrl() {
  const platformStr = getPlatform();
  const ext = process.platform === 'win32' ? '.exe' : '';
  return `https://github.com/${REPO}/releases/download/v${VERSION}/diary-header-${platformStr}${ext}`;
}

function getBinaryPath() {
  const ext = process.platform === 'win32' ? '.exe' : '';
  return path.join(__dirname, 'bin', `diary-header${ext}`);
}

function downloadFile(url, dest) {
  return new Promise((resolve, reject) => {
    console.log(`Downloading from ${url}...`);

    https.get(url, {
      headers: { 'User-Agent': 'diary-header-installer' }
    }, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        return downloadFile(response.headers.location, dest).then(resolve).catch(reject);
      }

      if (response.statusCode !== 200) {
        reject(new Error(`Failed to download: ${response.statusCode} ${response.statusMessage}`));
        return;
      }

      const file = fs.createWriteStream(dest);
      response.pipe(file);

      file.on('finish', () => {
        file.close();
        fs.chmodSync(dest, 0o755);
        console.log('Download complete!');
        resolve();
      });
    }).on('error', (err) => {
      fs.unlink(dest, () => {});
      reject(err);
    });
  });
}

async function install() {
  try {
    const binDir = path.join(__dirname, 'bin');
    if (!fs.existsSync(binDir)) {
      fs.mkdirSync(binDir, { recursive: true });
    }

    const binaryPath = getBinaryPath();
    const downloadUrl = getDownloadUrl();

    console.log('Installing diary-header...');
    console.log(`Platform: ${getPlatform()}`);

    await downloadFile(downloadUrl, binaryPath);

    console.log('diary-header installed successfully!');
    console.log('Run "diary-header" to get started.');
  } catch (error) {
    console.error('Installation failed:', error.message);
    console.error('\nFallback: Attempting to build from source...');

    try {
      execSync('cargo --version', { stdio: 'ignore' });
      console.log('Building with Cargo...');
      execSync('cargo build --release', { stdio: 'inherit', cwd: __dirname });

      const ext = process.platform === 'win32' ? '.exe' : '';
      const sourcePath = path.join(__dirname, 'target', 'release', `diary-header${ext}`);
      const binaryPath = getBinaryPath();

      fs.copyFileSync(sourcePath, binaryPath);
      fs.chmodSync(binaryPath, 0o755);

      console.log('Built from source successfully!');
    } catch (buildError) {
      console.error('\nBuild from source also failed.');
      console.error('Please ensure Rust and Cargo are installed: https://rustup.rs/');
      process.exit(1);
    }
  }
}

install();
