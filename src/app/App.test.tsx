import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useWorkspaceUi } from '../state/workspace';
import { App } from './App';

afterEach(() => {
  cleanup();
  useWorkspaceUi.setState({ active: 'playbook' });
});

const renderApp = () =>
  render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <App />
    </QueryClientProvider>,
  );

describe('App', () => {
  it('shows a friendly message when the core is unreachable (browser preview)', async () => {
    renderApp();
    expect(await screen.findByText(/Running in a browser preview/)).toBeTruthy();
  });

  it('switches workspace with ⌘2 and re-tints the window on the same render', () => {
    renderApp();
    const win = screen.getByTestId('window');
    expect(win.dataset.workspace).toBe('playbook');
    fireEvent.keyDown(window, { key: '2', metaKey: true });
    expect(win.dataset.workspace).toBe('lauf');
  });
});
