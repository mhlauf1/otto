/** Which half of the mark a workspace lights: the first two workspaces own a half each. */
export const sideFor = (index: number) => (index === 0 ? 'left' : index === 1 ? 'right' : 'none');
