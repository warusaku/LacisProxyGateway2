import { NextRequest, NextResponse } from 'next/server';

// basePath from next.config.mjs
const BASE_PATH = '/LacisProxyGateway2';

export function middleware(request: NextRequest) {
  const session = request.cookies.get('lpg_session');
  const { pathname } = request.nextUrl;

  // request.nextUrl.pathname includes basePath, so strip it for route logic
  const path = pathname.startsWith(BASE_PATH)
    ? pathname.slice(BASE_PATH.length) || '/'
    : pathname;

  const isLoginPage = path === '/login';
  const isApiRoute = path.startsWith('/api');
  const isStaticAsset = path.startsWith('/_next') || path.includes('.');

  // Skip middleware for API routes and static assets
  if (isApiRoute || isStaticAsset) {
    return NextResponse.next();
  }

  // No session + not login page → redirect to login
  if (!session && !isLoginPage) {
    return NextResponse.redirect(new URL(`${BASE_PATH}/login`, request.url));
  }

  // Has session + login page → redirect to dashboard
  if (session && isLoginPage) {
    return NextResponse.redirect(new URL(BASE_PATH, request.url));
  }

  return NextResponse.next();
}

// No config.matcher export: middleware runs on ALL requests.
// Filtering is done inside the function to avoid basePath + matcher regex issues
// where the generated regex fails to match the root URL without trailing slash.
