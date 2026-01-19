/**
 * Common metadata item interface for Tags and Groups
 */
export interface MetadataItem {
  id: string;
  name: string;
  sort: number;
  created_at: string;
  updated_at: string;
}
