// React Demo — entry point.
// Renders the App component into #root.

import { createRoot } from 'react-dom/client';
import { App } from './App';

const root = document.getElementById('root');
if (root) {
  createRoot(root).render(<App />);
}
