'use client';

import { useEffect } from 'react';
import { useRouter } from 'next/navigation';

export default function RoutesRedirectPage() {
  const router = useRouter();

  useEffect(() => {
    // router.push auto-prepends basePath
    router.replace('/server-routes');
  }, [router]);

  return (
    <div className="flex items-center justify-center h-64">
      <div className="text-gray-400">Redirecting to ServerRoutes...</div>
    </div>
  );
}
