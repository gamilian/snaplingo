export function mergeTimestampedPage<T extends { timestamp: number }>(
  groups: T[][],
  offset: number,
  limit: number,
) {
  return groups
    .flat()
    .sort((left, right) => right.timestamp - left.timestamp)
    .slice(offset, offset + limit);
}
