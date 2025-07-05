<script lang="ts">
    import { enhance } from '$app/forms'
    import { Button } from '$lib/components/ui/button/index.js'
    import { Input } from '$lib/components/ui/input/index.js'
    import { Label } from '$lib/components/ui/label/index.js'
    import {
        Card,
        CardContent,
        CardDescription,
        CardHeader,
        CardTitle,
    } from '$lib/components/ui/card/index.js'
    import type { ActionData } from './$types.js'

    export let form: ActionData

    let loading = false
</script>

<svelte:head>
    <title>Login - Clio</title>
</svelte:head>

<Card class="w-full">
    <CardHeader class="text-center">
        <CardTitle class="text-2xl">Welcome back</CardTitle>
        <CardDescription>Sign in to your Clio account</CardDescription>
    </CardHeader>
    <CardContent>
        <form
            method="POST"
            use:enhance={() => {
                loading = true
                return async ({ update }) => {
                    loading = false
                    await update()
                }
            }}
            class="space-y-4"
        >
            {#if form?.error}
                <div class="bg-destructive/10 rounded-md p-4">
                    <div class="text-destructive text-sm">
                        {form.error}
                    </div>
                </div>
            {/if}

            <div class="space-y-2">
                <Label for="email">Email</Label>
                <Input
                    id="email"
                    name="email"
                    type="email"
                    placeholder="Enter your email"
                    value={form?.email ?? ''}
                    required
                    disabled={loading}
                />
            </div>

            <div class="space-y-2">
                <Label for="password">Password</Label>
                <Input
                    id="password"
                    name="password"
                    type="password"
                    placeholder="Enter your password"
                    required
                    disabled={loading}
                />
            </div>

            <Button type="submit" class="w-full" disabled={loading}>
                {loading ? 'Signing in...' : 'Sign in'}
            </Button>
        </form>

        <div class="mt-6 space-y-4">
            <div class="relative">
                <div class="absolute inset-0 flex items-center">
                    <div class="border-muted w-full border-t"></div>
                </div>
                <div class="relative flex justify-center text-xs uppercase">
                    <span class="bg-card text-muted-foreground px-2">Or</span>
                </div>
            </div>

            <Button
                variant="outline"
                class="w-full"
                onclick="window.location.href='/auth/request-magic-link'"
            >
                <svg class="mr-2 h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M3 8l7.89 4.26a2 2 0 002.22 0L21 8M5 19h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"
                    />
                </svg>
                Sign in with Email Link
            </Button>
        </div>

        <div class="mt-6 text-center text-sm">
            <span class="text-muted-foreground">Don't have an account?</span>
            <a href="/signup" class="text-foreground hover:text-foreground/80 font-medium">
                Sign up
            </a>
        </div>
    </CardContent>
</Card>
