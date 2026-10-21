import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { LiveSession } from './registry';
import { UpNext } from './UpNext';

afterEach(cleanup);

const focus = vi.fn();
const session = {
  info: {
    id: '7de656b9-96e1-4226-9fbe-c8d129813f94',
    workspaceId: 'lauf',
    agent: 'claude',
    title: 'Claude Code',
    signIn: false,
    projectPath: '/stoc',
    branch: null,
  },
  term: { focus },
  exit: undefined,
  status: 'running',
} as unknown as LiveSession;

const renderUpNext = (onParallel?: (prompt: string) => Promise<void>) =>
  render(
    <QueryClientProvider client={new QueryClient()}>
      <UpNext workspaceId="lauf" session={session} onParallel={onParallel} />
    </QueryClientProvider>,
  );

describe('UpNext', () => {
  it('stays a single quiet button until Mike adds something', () => {
    renderUpNext();
    expect(screen.getByRole('button', { name: '+ Up next' })).toBeTruthy();
    expect(screen.queryByRole('textbox')).toBeNull();
  });

  it('adds with Enter, cancels with Escape, and offers parallel only in a project', () => {
    renderUpNext(vi.fn(() => Promise.resolve()));
    fireEvent.click(screen.getByRole('button', { name: '+ Up next' }));
    const field = screen.getByRole('textbox', { name: 'Next prompt' });
    expect(screen.getByRole('button', { name: 'Run in parallel' })).toBeTruthy();
    fireEvent.keyDown(field, { key: 'Escape' });
    expect(screen.queryByRole('textbox')).toBeNull();
    expect(focus).toHaveBeenCalled();
  });

  it('runs a task in parallel with ⌘↵', async () => {
    const onParallel = vi.fn(() => Promise.resolve());
    renderUpNext(onParallel);
    fireEvent.click(screen.getByRole('button', { name: '+ Up next' }));
    const field = screen.getByRole('textbox', { name: 'Next prompt' });
    fireEvent.change(field, { target: { value: 'update the footer copy' } });
    fireEvent.keyDown(field, { key: 'Enter', metaKey: true });
    await vi.waitFor(() => {
      expect(onParallel).toHaveBeenCalledWith('update the footer copy');
    });
  });

  it('has no parallel option outside a project', () => {
    renderUpNext(undefined);
    fireEvent.click(screen.getByRole('button', { name: '+ Up next' }));
    expect(screen.queryByRole('button', { name: 'Run in parallel' })).toBeNull();
  });
});
