<script lang="ts">
    import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card/index.js'
    import { Button } from '$lib/components/ui/button/index.js'
    import type { PageData } from './$types.js'
    import Input from '$lib/components/ui/input/input.svelte'
    import { Search } from '@lucide/svelte'

    export let data: PageData
</script>

<svelte:head>
    <title>Clio - Enterprise Search</title>
</svelte:head>

<div class="container mx-auto px-4">
    <!-- Centered Search Section -->
    <div class="min-h-[60vh] flex flex-col items-center justify-center">
        <div class="text-center mb-8">
            <h1 class="text-foreground text-4xl font-bold mb-4">Welcome to Clio</h1>
            <p class="text-muted-foreground text-lg">
                Your unified enterprise search platform. Search across all your connected data sources.
            </p>
        </div>

        <!-- Search Box -->
        <div class="w-full max-w-2xl">
            <div class="flex items-center bg-white border border-gray-300 rounded-full shadow-lg">
                <div class="pl-6 pr-3">
                    <Search class="h-5 w-5 text-gray-400" />
                </div>
                <Input 
                    type="text" 
                    placeholder="Ask anything..." 
                    class="flex-1 border-none bg-transparent py-4 px-0 focus:ring-0 focus:outline-none focus:border-none focus-visible:ring-0 focus-visible:ring-offset-0 rounded-full shadow-none"
                />
                <Button class="m-2 px-6 py-2 rounded-full cursor-pointer">Go</Button>
            </div>
        </div>
    </div>

    <!-- Quick Stats -->
    <div class="py-8">
        <div class="grid grid-cols-1 gap-6 md:grid-cols-3">
        <Card>
            <CardHeader>
                <CardTitle class="text-lg">Connected Sources</CardTitle>
            </CardHeader>
            <CardContent>
                <div class="text-foreground text-2xl font-bold">0</div>
                <p class="text-muted-foreground text-sm">Data sources connected</p>
            </CardContent>
        </Card>

        <Card>
            <CardHeader>
                <CardTitle class="text-lg">Indexed Documents</CardTitle>
            </CardHeader>
            <CardContent>
                <div class="text-foreground text-2xl font-bold">0</div>
                <p class="text-muted-foreground text-sm">Documents ready to search</p>
            </CardContent>
        </Card>

        <Card>
            <CardHeader>
                <CardTitle class="text-lg">Recent Searches</CardTitle>
            </CardHeader>
            <CardContent>
                <div class="text-foreground text-2xl font-bold">0</div>
                <p class="text-muted-foreground text-sm">Searches performed today</p>
            </CardContent>
        </Card>
        </div>
    </div>

    {#if data.user.role === 'admin'}
        <div class="mt-8">
            <Card>
                <CardHeader>
                    <CardTitle>Admin Quick Actions</CardTitle>
                </CardHeader>
                <CardContent class="space-y-4">
                    <div class="flex flex-wrap gap-4">
                        <a href="/admin/users">
                            <Button variant="outline">Manage Users</Button>
                        </a>
                        <Button variant="outline">Connect Data Sources</Button>
                        <Button variant="outline">View Analytics</Button>
                    </div>
                </CardContent>
            </Card>
        </div>
    {/if}
</div>
