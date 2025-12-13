#!/usr/bin/env bun
/**
 * Bun Native HTTP Server
 *
 * Direct Bun HTTP server for comparison
 */

const PORT = parseInt(process.env.PORT || '3003');

const server = Bun.serve({
    port: PORT,
    hostname: '0.0.0.0',

    fetch(req: Request): Response | Promise<Response> {
        const url = new URL(req.url);
        const path = url.pathname;
        const method = req.method;

        // Hello World
        if (method === 'GET' && path === '/') {
            return new Response('Hello, World!');
        }

        // Health check
        if (method === 'GET' && path === '/health') {
            return Response.json({ status: 'ok' });
        }

        // JSON API - GET with dynamic parameter
        const userMatch = path.match(/^\/api\/users\/([^/]+)$/);
        if (method === 'GET' && userMatch) {
            const id = userMatch[1];
            return Response.json({
                id,
                name: 'John Doe',
                email: `user${id}@example.com`,
                role: 'user'
            });
        }

        // JSON API - List
        if (method === 'GET' && path === '/api/users') {
            return Response.json({
                users: Array.from({ length: 10 }, (_, i) => ({
                    id: i + 1,
                    name: `User ${i + 1}`,
                    email: `user${i + 1}@example.com`
                })),
                total: 10,
                page: 1
            });
        }

        // POST with body parsing
        if (method === 'POST' && path === '/api/users') {
            return req.json().then(body => {
                return Response.json({
                    message: 'User created',
                    user: body,
                    id: Math.floor(Math.random() * 10000)
                });
            });
        }

        // Nested parameters
        const nestedMatch = path.match(/^\/api\/users\/([^/]+)\/posts\/([^/]+)$/);
        if (method === 'GET' && nestedMatch) {
            const [, userId, postId] = nestedMatch;
            return Response.json({
                userId,
                postId,
                title: 'Sample Post',
                content: 'Lorem ipsum dolor sit amet'
            });
        }

        // 404
        return new Response('Not Found', { status: 404 });
    }
});

console.log(`🥟 Bun HTTP server running on http://localhost:${PORT}`);
