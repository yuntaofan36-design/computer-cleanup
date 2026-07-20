import { useEffect, useRef, useState } from 'react';
import { loadAppIcon } from '../native';

export interface ApplicationIconProps {
  appId: string;
  name: string;
}

interface LoadedIcon {
  appId: string;
  dataUrl: string | null;
}

function fallbackLabel(name: string): string {
  return Array.from(name.trim())[0]?.toLocaleUpperCase() ?? '?';
}

export function ApplicationIcon({ appId, name }: ApplicationIconProps): JSX.Element {
  const containerRef = useRef<HTMLSpanElement>(null);
  const [loadedIcon, setLoadedIcon] = useState<LoadedIcon | null>(null);
  const [failedDataUrl, setFailedDataUrl] = useState<string | null>(null);

  useEffect(() => {
    const target = containerRef.current;
    let disposed = false;

    const requestIcon = (): void => {
      void loadAppIcon(appId)
        .then((dataUrl) => {
          if (!disposed) setLoadedIcon({ appId, dataUrl });
        })
        .catch(() => {
          if (!disposed) setLoadedIcon({ appId, dataUrl: null });
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
  }, [appId]);

  const dataUrl = loadedIcon?.appId === appId ? loadedIcon.dataUrl : null;
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
