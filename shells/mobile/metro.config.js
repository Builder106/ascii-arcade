const { getDefaultConfig } = require('expo/metro-config');
const path = require('path');

const config = getDefaultConfig(__dirname);

// Resolve the local aa-engine module from modules/
config.resolver.nodeModulesPaths = [
  path.resolve(__dirname, 'node_modules'),
];

config.watchFolders = [
  path.resolve(__dirname, 'modules/aa-engine/src'),
];

module.exports = config;
