import { describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { act } from 'react';
import userEvent from '@testing-library/user-event';
import { GeneratorForm } from '../../src/components/GeneratorForm';

describe('GeneratorForm', () => {
  it('submits sanitized generator arguments', async () => {
    const user = userEvent.setup();
    const commands = [
      { command: 'model', summary: 'Generate a model' },
      { command: 'migration', summary: 'Generate a migration' },
    ];
    const onSubmit = vi.fn();

    await act(async () => {
      render(<GeneratorForm commands={commands} onSubmit={onSubmit} />);
    });
    await screen.findByRole('combobox');
    await waitFor(() => expect(screen.getByRole('combobox')).toHaveValue('model'));

    await act(async () => {
      await user.type(screen.getByLabelText('Arguments'), ' Post title:string  ');
    });
    await act(async () => {
      await user.click(screen.getByRole('button', { name: /run generator/i }));
    });

    await waitFor(() =>
      expect(onSubmit).toHaveBeenCalledWith({ generator: 'model', arguments: ['Post', 'title:string'] })
    );
  });

  it('disables controls when no commands are available', async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();

    await act(async () => {
      render(<GeneratorForm commands={[]} onSubmit={onSubmit} />);
    });
    await screen.findByRole('combobox');

    expect(screen.getByRole('combobox')).toBeDisabled();
    expect(screen.getByRole('button', { name: /run generator/i })).toBeDisabled();

    await act(async () => {
      await user.click(screen.getByRole('button', { name: /run generator/i }));
    });
    await waitFor(() => expect(onSubmit).not.toHaveBeenCalled());
  });
});
