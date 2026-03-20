// Cytoscape style definitions for the graph view
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function getGraphStyles(dark: boolean): any[] {
	return [
		{
			selector: 'node',
			style: {
				'background-color': dark ? '#a1a1aa' : '#a8a29e',
				'background-opacity': 0.9,
				label: 'data(label)',
				color: dark ? '#d4d4d8' : '#57534e',
				'font-size': 10,
				'font-weight': 'normal',
				'text-valign': 'bottom',
				'text-margin-y': 5,
				width: 8,
				height: 8,
				'border-width': 0,
				'overlay-opacity': 0
			}
		},
		{
			selector: 'node[node_type = "tag"]',
			style: {
				'background-color': '#8b5cf6',
				width: 7,
				height: 7,
				shape: 'diamond'
			}
		},
		{
			selector: 'node[node_type = "attachment"]',
			style: {
				'background-color': '#22c55e',
				width: 5,
				height: 5,
				shape: 'round-rectangle'
			}
		},
		{
			selector: 'edge',
			style: {
				'line-color': dark ? '#3f3f46' : '#d6d3d1',
				width: 0.5,
				'curve-style': 'straight',
				'target-arrow-shape': 'triangle',
				'target-arrow-color': dark ? '#3f3f46' : '#d6d3d1',
				'arrow-scale': 0.6,
				'overlay-opacity': 0,
				opacity: 0.6
			}
		},
		{
			selector: 'node:active',
			style: { 'overlay-opacity': 0 }
		},
		{
			selector: 'node:grabbed',
			style: {
				'background-color': '#7c3aed',
				width: 12,
				height: 12
			}
		},
		{
			selector: '.hover-node',
			style: {
				'background-color': '#7c3aed',
				width: 12,
				height: 12,
				color: dark ? '#fafafa' : '#1c1917',
				'font-size': 12,
				'font-weight': 'bold'
			}
		},
		{
			selector: '.hover-neighbor',
			style: {
				'background-color': '#a78bfa',
				width: 10,
				height: 10,
				color: dark ? '#d4d4d8' : '#44403c',
				'font-weight': 'normal'
			}
		},
		{
			selector: '.hover-edge',
			style: {
				'line-color': '#8b5cf6',
				'target-arrow-color': '#8b5cf6',
				width: 1.5,
				opacity: 1
			}
		},
		{
			selector: '.dimmed',
			style: { opacity: 0.15 }
		}
	];
}
