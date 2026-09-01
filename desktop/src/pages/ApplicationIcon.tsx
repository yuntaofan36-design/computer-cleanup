import { useEffect, useRef, useState } from 'react';
import { loadAppIcon, loadStartupIcon } from '../native';

export interface ApplicationIconProps {
  appId: string;
  name: string;
}

interface LoadedIcon {
  id: string;
  dataUrl: string | null;
}

interface NativeApplicationIconProps {
  id: string;
  name: string;
  loadIcon: (id: string) => Promise<string | null>;
}

function fallbackLabel(name: string): string {
  return Array.from(name.trim())[0]?.toLocaleUpperCase() ?? '?';
}

function NativeApplicationIcon({ id, name, loadIcon }: NativeApplicationIconProps): JSX.Element {
  const containerRef = useRef<HTMLSpanElement>(null);
  const [loadedIcon, setLoadedIcon] = useState<LoadedIcon | null>(null);
  const [failedDataUrl, setFailedDataUrl] = useState<string | null>(null);

  useEffect(() => {
    const target = containerRef.current;
    let disposed = false;

    const requestIcon = (): void => {
      void loadIcon(id)
        .then((dataUrl) => {
          if (!disposed) setLoadedIcon({ id, dataUrl });
        })
        .catch(() => {
          if (!disposed) setLoadedIcon({ id, dataUrl: null });
        });
    };

    if (!target || typeof IntersectionObserver === 'undefined') {
      requestIcon();
      return () => {
        disposed = true;
      };
    }

    const observer = new IntersectionObserver((entries) => {
      if (!entries.some((entry) => entry.isIntersecting)) return;
      observer.disconnect();
      requestIcon();
    }, { rootMargin: '180px' });
    observer.observe(target);

    return () => {
      disposed = true;
      observer.disconnect();
    };
  }, [id, loadIcon]);

  const dataUrl = loadedIcon?.id === id ? loadedIcon.dataUrl : null;
  const showImage = dataUrl !== null && failedDataUrl !== dataUrl;

  return (
    <span
      ref={containerRef}
      className={`app-icon ${showImage ? 'real' : 'fallback'}`}
      aria-hidden="true"
    >
      {showImage ? (
        <img
          src={dataUrl}
          alt=""
          decoding="async"
          draggable={false}
          onError={() => setFailedDataUrl(dataUrl)}
        />
      ) : fallbackLabel(name)}
    </span>
  );
}

export function ApplicationIcon({ appId, name }: ApplicationIconProps): JSX.Element {
  return <NativeApplicationIcon id={appId} name={name} loadIcon={loadAppIcon} />;
}

export interface StartupApplicationIconProps {
  startupId: string;
  name: string;
}

export function StartupApplicationIcon({ startupId, name }: StartupApplicationIconProps): JSX.Element {
  return <NativeApplicationIcon id={startupId} name={name} loadIcon={loadStartupIcon} />;
}
