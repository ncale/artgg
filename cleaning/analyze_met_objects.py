#!/usr/bin/env python3
"""
Met Museum Objects Analysis Script
Analyzes MetObjects.csv to identify artworks suitable for wallpaper generation
"""

import pandas as pd
import numpy as np
from collections import Counter
import sys
import matplotlib.pyplot as plt
import matplotlib
matplotlib.use('Agg')  # Non-interactive backend for saving plots


def load_data(csv_path='MetObjects.csv'):
    """Load the CSV file with proper handling for large datasets"""
    print(f"Loading {csv_path}...")
    try:
        df = pd.read_csv(csv_path, encoding='utf-8', low_memory=False)
        print(f"✓ Loaded {len(df):,} total objects")
        return df
    except Exception as e:
        print(f"✗ Error loading CSV: {e}")
        sys.exit(1)


def drop_unnecessary_columns(df):
    """Drop columns that aren't needed for wallpaper analysis"""
    print("\n=== DROPPING UNNECESSARY COLUMNS ===")

    # Map user-specified names to actual CSV column names
    columns_to_drop = [
        'Object Number',  # accessionNumber
        'AccessionYear',
        'Artist Wikidata URL',
        'Artist ULAN URL',
        'Dimensions',  # dimensionsParsed (keeping parsed version if it exists)
        'Credit Line',
        'Object Wikidata URL',
        'Is Timeline Work',
        'Gallery Number',
        'Tags AAT URL',
        'Tags Wikidata URL',
        'Repository'
    ]

    # Also check for exact user-specified names in case they exist
    columns_to_drop.extend([
        'accessionNumber', 'accessionYear', 'artistWikidata_URL',
        'artistULAN_URL', 'dimensionsParsed', 'measurements',
        'creditLine', 'objectWikidata_URL', 'isTimelineWork', 'GalleryNumber'
    ])

    # Only drop columns that actually exist
    existing_cols_to_drop = [col for col in columns_to_drop if col in df.columns]

    if existing_cols_to_drop:
        print(f"Dropping {len(existing_cols_to_drop)} columns:")
        for col in existing_cols_to_drop:
            print(f"  - {col}")
        df = df.drop(columns=existing_cols_to_drop)
        print(f"✓ Remaining columns: {len(df.columns)}")
    else:
        print("⚠ No matching columns found to drop")

    return df


def filter_future_dates(df, current_year=2025):
    """Filter out objects with end dates after the current year"""
    print(f"\n=== FILTERING FUTURE DATES (>{current_year}) ===")

    if 'Object End Date' not in df.columns:
        print("✗ 'Object End Date' column not found")
        return df

    # Convert to numeric
    end_dates = pd.to_numeric(df['Object End Date'], errors='coerce')

    # Count future-dated works
    future_mask = end_dates > current_year
    future_count = future_mask.sum()

    if future_count > 0:
        print(f"Found {future_count:,} works with dates > {current_year}")

        # Show some examples
        future_years = end_dates[future_mask].value_counts().sort_index()
        print(f"Year range: {int(end_dates[future_mask].min())} to {int(end_dates[future_mask].max())}")
        print(f"Years affected: {', '.join(str(int(y)) for y in sorted(future_years.index[:10]))}")

        # Filter them out
        df_filtered = df[~future_mask].copy()
        print(f"✓ Filtered out {future_count:,} future-dated objects")
        print(f"Remaining: {len(df_filtered):,} objects")
        return df_filtered
    else:
        print(f"✓ No objects with dates > {current_year} found")
        return df


def check_link_resource_pattern(df):
    """Check if Link Resource is equivalent to a constructed object URL"""
    print("\n=== CHECKING LINK RESOURCE PATTERN ===")

    if 'Link Resource' not in df.columns:
        print("✗ 'Link Resource' column not found")
        return

    if 'Object ID' not in df.columns:
        print("✗ 'Object ID' column not found")
        return

    # Sample some entries to check the pattern
    sample_df = df[df['Link Resource'].notna()].head(100)

    if len(sample_df) == 0:
        print("✗ No Link Resource entries found")
        return

    # Check if Link Resource follows pattern: http://www.metmuseum.org/art/collection/search/{Object ID}
    base_url = "http://www.metmuseum.org/art/collection/search/"

    matches = 0
    total = 0

    for idx, row in sample_df.iterrows():
        link = row['Link Resource']
        obj_id = row['Object ID']
        expected_url = f"{base_url}{obj_id}"

        total += 1
        if str(link) == expected_url:
            matches += 1

    print(f"Checked {total} sample objects")
    print(f"Link Resource matches pattern: {matches}/{total} ({matches/total*100:.1f}%)")

    if matches == total:
        print("✓ Link Resource is REDUNDANT - can be constructed from Object ID")
        print(f"  Pattern: {base_url}{{Object ID}}")
    elif matches > 0:
        print(f"⚠ Link Resource is PARTIALLY redundant ({matches/total*100:.1f}% match)")
    else:
        print("✗ Link Resource does NOT follow expected pattern")
        print(f"  Example Link Resource: {sample_df.iloc[0]['Link Resource']}")
        print(f"  Expected pattern: {base_url}{{Object ID}}")


def filter_objects_with_images(df):
    """
    Filter objects that have images available.
    Looks for image URL columns or infers from available data.
    """
    print("\n=== FILTERING OBJECTS WITH IMAGES ===")

    # Check for image-related columns
    image_cols = [col for col in df.columns if 'image' in col.lower() or 'has image' in col.lower()]

    if image_cols:
        print(f"Found image columns: {image_cols}")
        # If there's a 'Has Images' or similar boolean column
        has_images_col = next((col for col in image_cols if 'has' in col.lower()), None)
        if has_images_col:
            filtered = df[df[has_images_col] == True].copy()
            print(f"✓ Filtered by '{has_images_col}': {len(filtered):,} objects")
        else:
            # Filter by checking if image URL columns are not empty
            primary_img = next((col for col in image_cols if 'primary' in col.lower()), image_cols[0])
            filtered = df[df[primary_img].notna() & (df[primary_img] != '')].copy()
            print(f"✓ Filtered by non-empty '{primary_img}': {len(filtered):,} objects")
    else:
        print("⚠ No image columns found in CSV")
        print("Available columns:", df.columns.tolist()[:10], "...")
        print("\nAssuming all objects may have images (check via API)")
        filtered = df.copy()

    return filtered


def extract_future_dated_works(df, current_year=2026, output_file='future_dated_works.csv'):
    """Extract and save works with end dates in the future"""
    print(f"\n=== EXTRACTING FUTURE-DATED WORKS (>{current_year}) ===")

    if 'Object End Date' not in df.columns:
        print("✗ 'Object End Date' column not found")
        return

    # Convert to numeric and filter for future dates
    df_with_dates = df.copy()
    df_with_dates['Object End Date Numeric'] = pd.to_numeric(df_with_dates['Object End Date'], errors='coerce')

    future_works = df_with_dates[df_with_dates['Object End Date Numeric'] > current_year].copy()
    future_works = future_works.drop(columns=['Object End Date Numeric'])

    if len(future_works) == 0:
        print(f"✓ No works found with dates > {current_year}")
        return

    print(f"Found {len(future_works):,} works with end dates > {current_year}")

    # Show some statistics
    future_dates = pd.to_numeric(future_works['Object End Date'], errors='coerce')
    print(f"Date range: {int(future_dates.min())} to {int(future_dates.max())}")

    # Show sample of what these works are
    print(f"\nSample of future-dated works:")
    display_cols = ['Object ID', 'Title', 'Object End Date', 'Department', 'Classification', 'Object Name']
    available_cols = [col for col in display_cols if col in future_works.columns]

    sample = future_works[available_cols].head(10)
    for idx, row in sample.iterrows():
        title = str(row.get('Title', 'N/A')) if pd.notna(row.get('Title')) else 'N/A'
        dept = str(row.get('Department', 'N/A')) if pd.notna(row.get('Department')) else 'N/A'
        classification = str(row.get('Classification', row.get('Object Name', 'N/A')))
        if pd.isna(row.get('Classification')):
            classification = str(row.get('Object Name', 'N/A')) if pd.notna(row.get('Object Name')) else 'N/A'

        print(f"\n  Year {int(row['Object End Date'])}: {title[:60]}")
        print(f"    Dept: {dept}")
        print(f"    Type: {classification}")

    # Show year distribution
    year_counts = future_dates.value_counts().sort_index()
    print(f"\nYear distribution:")
    for year, count in year_counts.items():
        print(f"  {int(year)}: {count:,} works")

    # Save to CSV
    future_works.to_csv(output_file, index=False)
    print(f"\n✓ Saved {len(future_works):,} future-dated works to {output_file}")


def create_year_boxplot(df, output_file='work_years_boxplot.png'):
    """Create a boxplot visualization of Object End Date (work years)"""
    print("\n=== CREATING YEAR BOXPLOT ===")

    if 'Object End Date' not in df.columns:
        print("✗ 'Object End Date' column not found")
        return

    # Convert to numeric and remove invalid values
    years = pd.to_numeric(df['Object End Date'], errors='coerce').dropna()

    if len(years) == 0:
        print("✗ No valid year data found")
        return

    # Calculate IQR-based outliers
    Q1 = years.quantile(0.25)
    Q3 = years.quantile(0.75)
    IQR = Q3 - Q1
    lower_bound = Q1 - 1.5 * IQR
    upper_bound = Q3 + 1.5 * IQR

    # Filter outliers
    years_no_outliers = years[(years >= lower_bound) & (years <= upper_bound)]
    outliers = years[(years < lower_bound) | (years > upper_bound)]

    print(f"Total objects with dates: {len(years):,}")
    print(f"Date range (all): {int(years.min())} to {int(years.max())}")
    print(f"\nOutlier Calculation (IQR Method):")
    print(f"  Q1: {int(Q1)}")
    print(f"  Q3: {int(Q3)}")
    print(f"  IQR: {int(IQR)}")
    print(f"  Lower bound (Q1 - 1.5*IQR): {int(lower_bound)}")
    print(f"  Upper bound (Q3 + 1.5*IQR): {int(upper_bound)}")
    print(f"\nObjects without outliers: {len(years_no_outliers):,}")
    print(f"Outliers removed: {len(outliers):,} ({len(outliers)/len(years)*100:.2f}%)")
    print(f"Date range (no outliers): {int(years_no_outliers.min())} to {int(years_no_outliers.max())}")

    # Create figure with two subplots
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

    # Boxplot 1: All dates
    bp1 = ax1.boxplot([years], vert=True, patch_artist=True, widths=0.5)
    bp1['boxes'][0].set_facecolor('lightblue')
    bp1['boxes'][0].set_edgecolor('darkblue')
    bp1['medians'][0].set_color('red')
    bp1['medians'][0].set_linewidth(2)

    ax1.set_ylabel('Year', fontsize=12)
    ax1.set_title('All Object End Dates\n(with outliers)', fontsize=14, fontweight='bold')
    ax1.grid(axis='y', alpha=0.3, linestyle='--')
    ax1.set_xticklabels(['All Works'])

    # Add statistics text
    stats_text1 = f"N: {len(years):,}\nMedian: {int(years.median())}\nQ1: {int(Q1)}\nQ3: {int(Q3)}\nMin: {int(years.min())}\nMax: {int(years.max())}"
    ax1.text(1.15, years.median(), stats_text1, fontsize=9, verticalalignment='center',
             bbox=dict(boxstyle='round', facecolor='wheat', alpha=0.5))

    # Boxplot 2: Without outliers
    bp2 = ax2.boxplot([years_no_outliers], vert=True, patch_artist=True, widths=0.5)
    bp2['boxes'][0].set_facecolor('lightcoral')
    bp2['boxes'][0].set_edgecolor('darkred')
    bp2['medians'][0].set_color('blue')
    bp2['medians'][0].set_linewidth(2)

    ax2.set_ylabel('Year', fontsize=12)
    ax2.set_title(f'Object End Dates\n(outliers removed: {len(outliers):,})', fontsize=14, fontweight='bold')
    ax2.grid(axis='y', alpha=0.3, linestyle='--')
    ax2.set_xticklabels(['No Outliers'])

    # Add statistics text
    stats_text2 = f"N: {len(years_no_outliers):,}\nMedian: {int(years_no_outliers.median())}\nQ1: {int(years_no_outliers.quantile(0.25))}\nQ3: {int(years_no_outliers.quantile(0.75))}\nMin: {int(years_no_outliers.min())}\nMax: {int(years_no_outliers.max())}"
    ax2.text(1.15, years_no_outliers.median(), stats_text2, fontsize=9, verticalalignment='center',
             bbox=dict(boxstyle='round', facecolor='lightgreen', alpha=0.5))

    plt.suptitle('Met Museum Collection: Object End Date Distribution (IQR Method)', fontsize=16, fontweight='bold', y=1.02)
    plt.tight_layout()

    # Save the plot
    plt.savefig(output_file, dpi=300, bbox_inches='tight')
    print(f"✓ Boxplot saved to {output_file}")
    plt.close()


def analyze_frequency(df, column_name, top_n=20):
    """Perform frequency analysis on a column"""
    print(f"\n--- {column_name} ---")

    if column_name not in df.columns:
        print(f"✗ Column '{column_name}' not found")
        return

    # Handle NaN values
    valid_data = df[column_name].dropna()
    total_valid = len(valid_data)
    total_missing = len(df) - total_valid

    print(f"Valid entries: {total_valid:,} | Missing: {total_missing:,}")

    if total_valid == 0:
        return

    # Get frequency counts
    if column_name == 'Tags':
        # Special handling for pipe-delimited tags
        all_tags = []
        for tags_str in valid_data:
            if isinstance(tags_str, str) and tags_str:
                all_tags.extend([tag.strip() for tag in tags_str.split('|')])

        if all_tags:
            tag_counts = Counter(all_tags)
            print(f"\nTotal unique tags: {len(tag_counts):,}")
            print(f"Total tag occurrences: {len(all_tags):,}")
            print(f"\nTop {top_n} tags:")
            for tag, count in tag_counts.most_common(top_n):
                pct = (count / len(all_tags)) * 100
                print(f"  {tag:30s} : {count:6,} ({pct:5.2f}%)")
        else:
            print("No tags found")
    else:
        # Regular column frequency
        value_counts = valid_data.value_counts()
        print(f"\nUnique values: {len(value_counts):,}")
        print(f"\nTop {top_n} values:")
        for value, count in value_counts.head(top_n).items():
            pct = (count / total_valid) * 100
            display_val = str(value)[:50]  # Truncate long values
            print(f"  {display_val:30s} : {count:6,} ({pct:5.2f}%)")


def main():
    print("=" * 70)
    print("MET MUSEUM WALLPAPER SOURCE ANALYSIS")
    print("=" * 70)

    # Load data
    df = load_data('MetObjects.csv')

    # Drop unnecessary columns
    df = drop_unnecessary_columns(df)

    # Check if Link Resource is redundant
    check_link_resource_pattern(df)

    # Extract future-dated works before filtering them out
    extract_future_dated_works(df, current_year=2026)

    # Filter out future-dated objects
    df = filter_future_dates(df, current_year=2025)

    # Filter for objects with images
    df_with_images = filter_objects_with_images(df)

    print(f"\n{'=' * 70}")
    print(f"FINAL DATASET: {len(df_with_images):,} objects suitable for wallpapers")
    print(f"Reduction: {len(df) - len(df_with_images):,} objects filtered out")
    print(f"{'=' * 70}")

    # Frequency analysis on key columns
    print("\n" + "=" * 70)
    print("FREQUENCY ANALYSIS")
    print("=" * 70)

    columns_to_analyze = [
        'Is Highlight',
        'Tags',
        'Object End Date',
        'Department',
        'Classification'
    ]

    for col in columns_to_analyze:
        analyze_frequency(df_with_images, col)

    # Create boxplot visualization
    create_year_boxplot(df_with_images)

    # Summary statistics
    print(f"\n{'=' * 70}")
    print("SUMMARY STATISTICS")
    print(f"{'=' * 70}")

    if 'Is Highlight' in df_with_images.columns:
        highlights = df_with_images['Is Highlight'].sum() if df_with_images['Is Highlight'].dtype == bool else \
                     (df_with_images['Is Highlight'] == True).sum()
        print(f"Highlighted objects: {highlights:,} ({highlights/len(df_with_images)*100:.2f}%)")

    if 'Object End Date' in df_with_images.columns:
        dates = pd.to_numeric(df_with_images['Object End Date'], errors='coerce').dropna()
        if len(dates) > 0:
            print(f"\nDate range: {int(dates.min())} to {int(dates.max())}")
            print(f"Median year: {int(dates.median())}")

    if 'Tags' in df_with_images.columns:
        tagged = df_with_images['Tags'].notna().sum()
        print(f"\nObjects with tags: {tagged:,} ({tagged/len(df_with_images)*100:.2f}%)")

    # Save filtered dataset info
    print(f"\n{'=' * 70}")
    output_file = 'filtered_objects.csv'
    print(f"Saving filtered dataset to {output_file}...")
    df_with_images.to_csv(output_file, index=False)
    print(f"✓ Saved {len(df_with_images):,} objects")
    print("=" * 70)


if __name__ == '__main__':
    main()
