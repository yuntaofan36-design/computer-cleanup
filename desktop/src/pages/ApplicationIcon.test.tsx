// @vitest-environment jsdom

import '@testing-library/jest-dom/vitest';
import { fireEvent, render, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { loadAppIcon } from '../native';
import { ApplicationIcon } from './ApplicationIcon';

vi.mock('../native', () => ({
  loadAppIcon: vi.fn(),
}));

const mockedLoadAppIcon = vi.mocked(loadAppIcon);
const iconDataUrl = 'data:image/png;base64,iVBORw0KGgo=';

describe('ApplicationIcon', () => {
  beforeEach(() => {
    vi.stubGlobal('IntersectionObserver', undefined);
    mockedLoadAppIcon.mockReset();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('renders a native application icon without changing the fixed slot', async () => {
    mockedLoadAppIcon.mockResolvedValue(iconDataUrl);
    const { container } = render(<ApplicationIcon appId="app_1" name="Visual Studio Code" />);

    await waitFor(() => expect(container.querySelector('img')).not.toBeNull());

    const slot = container.querySelector('.app-icon');
    const image = container.querySelector('img');
    expect(slot).toHaveClass('real');
    expect(image).toHaveAttribute('src', iconDataUrl);
    expect(mockedLoadAppIcon).toHaveBeenCalledWith('app_1');
  });

  it('falls back to the application initial when image decoding fails', async () => {
    mockedLoadAppIcon.mockResolvedValue(iconDataUrl);
    const { container, getByText } = render(
      <ApplicationIcon appId="app_2" name="Figma" />,
    );

    await waitFor(() => expect(container.querySelector('img')).not.toBeNull());
    fireEvent.error(container.querySelector('img') as HTMLImageElement);

    expect(container.querySelector('img')).toBeNull();
    expect(container.querySelector('.app-icon')).toHaveClass('fallback');
    expect(getByText('F')).toBeInTheDocument();
  });

  it('uses a safe placeholder when no icon or name is available', async () => {
    mockedLoadAppIcon.mockResolvedValue(null);
    const { container, getByText } = render(<ApplicationIcon appId="app_3" name="" />);

    await waitFor(() => expect(mockedLoadAppIcon).toHaveBeenCalledWith('app_3'));

    expect(container.querySelector('img')).toBeNull();
    expect(container.querySelector('.app-icon')).toHaveClass('fallback');
    expect(getByText('?')).toBeInTheDocument();
  });
});
