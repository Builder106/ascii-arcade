import { Stack } from 'expo-router';
import { StatusBar } from 'expo-status-bar';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { ArcadeProvider } from '../context/ArcadeContext';

export default function RootLayout() {
  return (
    <SafeAreaProvider>
      <ArcadeProvider>
        <StatusBar style="light" hidden />
        <Stack screenOptions={{ headerShown: false }} />
      </ArcadeProvider>
    </SafeAreaProvider>
  );
}
