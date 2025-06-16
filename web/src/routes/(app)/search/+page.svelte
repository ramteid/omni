<script lang="ts">
    import { page } from '$app/stores'
    import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card/index.js'
    import { Button } from '$lib/components/ui/button/index.js'
    import { Input } from '$lib/components/ui/input/index.js'
    import { Search, FileText, Calendar, User, Filter } from '@lucide/svelte'
    import type { PageData } from './$types.js'
    import type { SearchResponse } from '$lib/types/search.js'

    let { data }: { data: PageData } = $props()

    let searchQuery = $state('')
    
    $effect(() => {
        searchQuery = $page.url.searchParams.get('q') || ''
    })
    let isLoading = $state(false)
    let selectedSources = $state<Set<string>>(new Set())

    const sourceDisplayNames: Record<string, string> = {
        'google_drive': 'Google Drive',
        'google_docs': 'Google Docs',
        'google_sheets': 'Google Sheets',
        'gmail': 'Gmail',
        'slack': 'Slack',
        'confluence': 'Confluence',
        'jira': 'JIRA',
        'github': 'GitHub'
    }

    let sourceFacets = $derived(data.searchResults ? extractSourceFacets(data.searchResults) : [])
    let filteredResults = $derived(data.searchResults ? filterResults(data.searchResults, selectedSources) : null)

    interface SourceFacet {
        source: string
        displayName: string
        count: number
    }

    function extractSourceFacets(searchResults: SearchResponse): SourceFacet[] {
        const sourceCounts = new Map<string, number>()
        
        searchResults.results.forEach(result => {
            const source = result.document.source
            sourceCounts.set(source, (sourceCounts.get(source) || 0) + 1)
        })
        
        return Array.from(sourceCounts.entries())
            .map(([source, count]) => ({
                source,
                displayName: sourceDisplayNames[source] || source,
                count
            }))
            .sort((a, b) => b.count - a.count)
    }

    function filterResults(searchResults: SearchResponse, selectedSources: Set<string>): SearchResponse {
        if (selectedSources.size === 0) {
            return searchResults
        }
        
        const filteredResults = searchResults.results.filter(result => 
            selectedSources.has(result.document.source)
        )
        
        return {
            ...searchResults,
            results: filteredResults,
            total_count: filteredResults.length
        }
    }

    function toggleSource(source: string) {
        if (selectedSources.has(source)) {
            selectedSources.delete(source)
        } else {
            selectedSources.add(source)
        }
        selectedSources = new Set(selectedSources)
    }

    function clearFilters() {
        selectedSources = new Set()
    }

    function handleSearch() {
        if (searchQuery.trim()) {
            window.location.href = `/search?q=${encodeURIComponent(searchQuery.trim())}`
        }
    }

    function handleKeyPress(event: KeyboardEvent) {
        if (event.key === 'Enter') {
            handleSearch()
        }
    }

    function formatDate(dateStr: string) {
        return new Date(dateStr).toLocaleDateString()
    }

    function truncateContent(content: string, maxLength: number = 200) {
        if (content.length <= maxLength) return content
        return content.substring(0, maxLength) + '...'
    }
</script>

<svelte:head>
    <title>Search Results - Clio</title>
</svelte:head>

<div class="container mx-auto px-4 py-6">
    <!-- Search Header -->
    <div class="mb-8">
        <div class="flex items-center gap-4 mb-4">
            <div class="flex items-center flex-1 rounded-lg border border-gray-300 bg-white shadow-sm">
                <div class="p-3">
                    <Search class="h-5 w-5 text-gray-400" />
                </div>
                <Input
                    type="text"
                    bind:value={searchQuery}
                    placeholder="Search across your organization..."
                    class="flex-1 border-none bg-transparent shadow-none focus:ring-0 focus-visible:ring-0"
                    onkeypress={handleKeyPress}
                />
                <Button 
                    class="m-2 px-6" 
                    onclick={handleSearch}
                    disabled={isLoading}
                >
                    {isLoading ? 'Searching...' : 'Search'}
                </Button>
            </div>
        </div>

        {#if filteredResults}
            <div class="text-sm text-gray-600">
                Found {filteredResults.total_count} results in {data.searchResults.query_time_ms}ms for "{data.searchResults.query}"
                {#if selectedSources.size > 0}
                    <span class="ml-2">• Filtered by {selectedSources.size} source{selectedSources.size > 1 ? 's' : ''}</span>
                {/if}
            </div>
        {/if}
    </div>

    <div class="flex gap-6">
        <!-- Search Results -->
        <div class="flex-1">
            {#if filteredResults}
                {#if filteredResults.results.length > 0}
                    <div class="space-y-4">
                        {#each filteredResults.results as result}
                    <Card class="hover:shadow-md transition-shadow">
                        <CardContent class="p-6">
                            <div class="flex items-start justify-between mb-3">
                                <div class="flex items-center gap-2">
                                    <FileText class="h-4 w-4 text-blue-600" />
                                    <h3 class="text-lg font-semibold text-blue-600 hover:text-blue-800">
                                        <a href={result.document.url || '#'} target="_blank" rel="noopener noreferrer">
                                            {result.document.title}
                                        </a>
                                    </h3>
                                </div>
                                <div class="flex items-center gap-4 text-sm text-gray-500">
                                    <div class="flex items-center gap-1">
                                        <User class="h-3 w-3" />
                                        {result.document.source}
                                    </div>
                                    <div class="flex items-center gap-1">
                                        <Calendar class="h-3 w-3" />
                                        {formatDate(result.document.created_at)}
                                    </div>
                                    <div class="px-2 py-1 bg-gray-100 rounded text-xs">
                                        Score: {result.score.toFixed(2)}
                                    </div>
                                </div>
                            </div>

                            <div class="text-gray-700 mb-3">
                                {truncateContent(result.document.content)}
                            </div>

                            {#if result.highlights.length > 0}
                                <div class="mb-3">
                                    <h4 class="text-sm font-medium text-gray-900 mb-2">Highlights:</h4>
                                    <div class="space-y-1">
                                        {#each result.highlights as highlight}
                                            <div class="text-sm text-gray-600 bg-yellow-50 p-2 rounded">
                                                {@html highlight}
                                            </div>
                                        {/each}
                                    </div>
                                </div>
                            {/if}

                            <div class="text-xs text-gray-500">
                                Match type: {result.match_type} • Content type: {result.document.content_type}
                            </div>
                        </CardContent>
                            </Card>
                        {/each}
                    </div>

                    <!-- Pagination -->
                    {#if data.searchResults.has_more}
                        <div class="mt-8 text-center">
                            <Button variant="outline">Load More Results</Button>
                        </div>
                    {/if}
                {:else}
                    <div class="text-center py-12">
                        <Search class="h-12 w-12 text-gray-400 mx-auto mb-4" />
                        <h3 class="text-lg font-medium text-gray-900 mb-2">No results found</h3>
                        <p class="text-gray-600 mb-4">
                            {#if selectedSources.size > 0}
                                No results found in the selected sources. Try clearing filters or adjusting your search.
                            {:else}
                                Try adjusting your search terms or check if your data sources are connected and indexed.
                            {/if}
                        </p>
                        {#if selectedSources.size > 0}
                            <Button variant="outline" onclick={clearFilters} class="mr-2">
                                Clear Filters
                            </Button>
                        {/if}
                        <Button variant="outline" onclick={() => window.location.href = '/'}>
                            Back to Home
                        </Button>
                    </div>
                {/if}
            {:else if $page.url.searchParams.get('q')}
                <div class="text-center py-12">
                    <div class="animate-spin h-8 w-8 border-4 border-gray-300 border-t-blue-600 rounded-full mx-auto mb-4"></div>
                    <p class="text-gray-600">Searching...</p>
                </div>
            {:else}
                <div class="text-center py-12">
                    <Search class="h-12 w-12 text-gray-400 mx-auto mb-4" />
                    <h3 class="text-lg font-medium text-gray-900 mb-2">Enter a search query</h3>
                    <p class="text-gray-600">
                        Search across your organization's documents, emails, and more.
                    </p>
                </div>
            {/if}
        </div>

        <!-- Facets Sidebar -->
        {#if data.searchResults && data.searchResults.results.length > 0}
            <div class="w-80">
                <Card>
                    <CardHeader>
                        <div class="flex items-center justify-between">
                            <CardTitle class="text-base flex items-center gap-2">
                                <Filter class="h-4 w-4" />
                                Filter by Source
                            </CardTitle>
                            {#if selectedSources.size > 0}
                                <Button 
                                    variant="ghost" 
                                    size="sm" 
                                    onclick={clearFilters}
                                    class="text-xs"
                                >
                                    Clear all
                                </Button>
                            {/if}
                        </div>
                    </CardHeader>
                    <CardContent class="space-y-3">
                        {#each sourceFacets as facet}
                            <label class="flex items-center justify-between cursor-pointer hover:bg-gray-50 p-2 rounded">
                                <div class="flex items-center gap-2">
                                    <input 
                                        type="checkbox"
                                        checked={selectedSources.has(facet.source)}
                                        onchange={() => toggleSource(facet.source)}
                                        class="h-4 w-4 text-blue-600 rounded border-gray-300 focus:ring-blue-500"
                                    />
                                    <span class="text-sm font-medium text-gray-700">
                                        {facet.displayName}
                                    </span>
                                </div>
                                <span class="text-xs text-gray-500 bg-gray-100 px-2 py-1 rounded">
                                    {facet.count}
                                </span>
                            </label>
                        {/each}
                    </CardContent>
                </Card>
            </div>
        {/if}
    </div>
</div>