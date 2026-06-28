'use strict';
// Expo config plugin: patches the generated Podfile so that the fmt library
// (bundled by the glog pod and used by React Native) compiles cleanly under
// Apple Clang.  Apple Clang is stricter than upstream LLVM about consteval
// constant-expression evaluation; defining FMT_CONSTEVAL= disables that code
// path in fmt and lets the build proceed.
const { withDangerousMod } = require('@expo/config-plugins');
const fs = require('fs');
const path = require('path');

const FMT_FIX = [
  '',
  '  # withGlogFmtFix: Apple Clang 16 is stricter about consteval than upstream LLVM.',
  '  # FMT_USE_CONSTEVAL=0 makes fmt use constexpr instead, bypassing the compile error.',
  '  # Must patch xcconfig files directly — project-level build_settings lose to xcconfig.',
  '  require "xcodeproj"',
  "  fmt_xcconfig_glob = Dir[File.join(installer.sandbox.root, 'Target Support Files', 'fmt', 'fmt.*.xcconfig')]",
  '  fmt_xcconfig_glob.each do |xcconfig_path|',
  '    content = File.read(xcconfig_path)',
  "    next if content.include?('FMT_USE_CONSTEVAL')",
  "    File.open(xcconfig_path, 'a') { |f| f.puts \"\\nOTHER_CPLUSPLUSFLAGS = $(inherited) -DFMT_USE_CONSTEVAL=0\" }",
  '  end',
].join('\n');

const withGlogFmtFix = (config) =>
  withDangerousMod(config, [
    'ios',
    async (modConfig) => {
      const podfilePath = path.join(
        modConfig.modRequest.platformProjectRoot,
        'Podfile',
      );
      let podfile = fs.readFileSync(podfilePath, 'utf8');

      if (!podfile.includes('FMT_CONSTEVAL')) {
        // Insert immediately after the opening of the post_install block.
        podfile = podfile.replace(
          'post_install do |installer|',
          'post_install do |installer|' + FMT_FIX,
        );
        fs.writeFileSync(podfilePath, podfile);
      }

      return modConfig;
    },
  ]);

module.exports = withGlogFmtFix;
