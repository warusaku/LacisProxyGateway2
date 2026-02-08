import { NextRequest, NextResponse } from 'next/server';

export function middleware(request: NextRequest) {
  const session = request.cookies.get('lpg_session');
  const { pathname } = request.nextUrl;

  const isLoginPage = pathname === '/LacisProxyGateway2/login';
  const isApiRoute = pathname.startsWith('/LacisProxyGateway2/api');
  const isStaticAsset =
    pathname.startsWith('/LacisProxyGateway2/_next') ||
    pathname.includes('.');

  // Skip middleware for API routes and static assets
  if (isApiRoute || isStaticAsset) {
    return NextResponse.next();
  }

  // No session + not login page → redirect to login
  if (!session && !isLoginPage) {
    return NextResponse.redirect(new URL('/LacisProxyGateway2/login', request.url));
  }

  // Has session + login page → redirect to dashboard
  if (session && isLoginPage) {
    return NextResponse.redirect(new URL('/LacisProxyGateway2', request.url));
  }

  return NextResponse.next();
}

export const config = {
  matcher: ['/LacisProxyGateway2/:path*'],
};
