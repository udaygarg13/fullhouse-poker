import { useState, useEffect } from 'react';
import { RotateCcw } from 'lucide-react';

export function LandscapeLock({ children }: { children: React.ReactNode }) {
  const [isPortrait, setIsPortrait] = useState(false);

  useEffect(() => {
    const checkOrientation = () => {
      const isMobile = window.innerWidth <= 768;
      const isPortraitMode = window.innerHeight > window.innerWidth;
      setIsPortrait(isMobile && isPortraitMode);
    };

    checkOrientation();
    window.addEventListener('resize', checkOrientation);
    window.addEventListener('orientationchange', checkOrientation);

    return () => {
      window.removeEventListener('resize', checkOrientation);
      window.removeEventListener('orientationchange', checkOrientation);
    };
  }, []);

  if (isPortrait) {
    return (
      <div className="landscape-lock">
        <RotateCcw className="icon text-emerald-500" />
        <h2>Rotate Your Device</h2>
        <p>Please rotate your device to landscape mode for the best experience.</p>
      </div>
    );
  }

  return <>{children}</>;
}
