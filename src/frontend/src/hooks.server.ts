import type { Handle } from '@sveltejs/kit';

const BACKEND_URL = process.env.BACKEND_URL || 'http://localhost:3000';

export const handle: Handle = async ({ event, resolve }) => {
	if (event.url.pathname.startsWith('/api/')) {
		const backendUrl = `${BACKEND_URL}${event.url.pathname}${event.url.search}`;

		try {
			const headers = new Headers();
			// Forward cookies and content-type
			const cookie = event.request.headers.get('cookie');
			if (cookie) headers.set('cookie', cookie);
			const contentType = event.request.headers.get('content-type');
			if (contentType) headers.set('content-type', contentType);
			headers.set('x-forwarded-for', event.getClientAddress());

			const response = await fetch(backendUrl, {
				method: event.request.method,
				headers,
				body:
					event.request.method !== 'GET' && event.request.method !== 'HEAD'
						? await event.request.text()
						: undefined,
				redirect: 'manual',
				signal: AbortSignal.timeout(30_000)
			});

			// Copy response headers, preserving set-cookie
			const responseHeaders = new Headers();
			response.headers.forEach((value, key) => {
				responseHeaders.append(key, value);
			});

			return new Response(response.body, {
				status: response.status,
				statusText: response.statusText,
				headers: responseHeaders
			});
		} catch (err) {
			console.error(`Proxy error: ${backendUrl}`, err);
			return new Response('Backend unavailable', { status: 502 });
		}
	}

	return resolve(event);
};
