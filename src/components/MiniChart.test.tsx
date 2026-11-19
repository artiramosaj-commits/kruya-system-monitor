import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/react';
import MiniChart from './MiniChart';

describe('MiniChart', () => {
  it('renders an SVG element', () => {
    const { container } = render(<MiniChart data={[10, 20, 30]} color="#3b82f6" />);
    const svg = container.querySelector('svg');
    expect(svg).not.toBeNull();
  });

  it('renders polyline when data has values', () => {
    const { container } = render(<MiniChart data={[10, 20, 30]} color="#3b82f6" />);
    const polyline = container.querySelector('polyline');
    expect(polyline).not.toBeNull();
  });

  it('returns null when data has fewer than 2 points', () => {
    const { container } = render(<MiniChart data={[]} color="#3b82f6" />);
    expect(container.querySelector('svg')).toBeNull();
    const { container: c2 } = render(<MiniChart data={[5]} color="#3b82f6" />);
    expect(c2.querySelector('svg')).toBeNull();
  });

  it('applies custom width and height', () => {
    const { container } = render(<MiniChart data={[1, 2, 3]} color="red" width={200} height={40} />);
    const svg = container.querySelector('svg')!;
    expect(svg.getAttribute('width')).toBe('200');
    expect(svg.getAttribute('height')).toBe('40');
  });

  it('uses default dimensions when not specified', () => {
    const { container } = render(<MiniChart data={[1, 2, 3]} color="blue" />);
    const svg = container.querySelector('svg')!;
    expect(svg.getAttribute('width')).toBe('80');
    expect(svg.getAttribute('height')).toBe('32');
  });

  it('renders correct number of points in polyline', () => {
    const data = [5, 10, 15, 20];
    const { container } = render(<MiniChart data={data} color="#22c55e" />);
    const polyline = container.querySelector('polyline');
    const points = polyline!.getAttribute('points');
    const pointCount = points!.trim().split(/\s+/).length;
    expect(pointCount).toBe(data.length);
  });
});
