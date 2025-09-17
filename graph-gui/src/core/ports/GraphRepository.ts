import { RawGraphSnapshot } from '../models/Graph';

export interface GraphRepository {
  fetchGraph(): Promise<RawGraphSnapshot>;
}
