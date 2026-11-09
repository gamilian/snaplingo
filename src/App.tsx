import { useEffect } from 'react';

// The browser build retains the previous React views as visual references.
// Desktop application composition and platform commands live in the Rust host.
export function referenceUrl(search: string) {
  const parameters = new URLSearchParams(search);
  const surface = parameters.get('surface');
  const reference = surface === 'permission'
    ? 'permission'
    : surface === 'result' || parameters.get('window') === 'capture-result'
      ? 'result'
      : 'settings';
  return `/crates/snaplingo-app/tests/native_ui/${reference}_reference.html${search}`;
}

export default function App() {
  useEffect(() => {
    window.location.replace(referenceUrl(window.location.search));
  }, []);
  return null;
}
