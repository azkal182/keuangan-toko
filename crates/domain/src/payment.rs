use chrono::NaiveDate;
use uuid::Uuid;

pub struct OpenDebit {
    pub entry_id: Uuid,
    pub effective_date: Option<NaiveDate>,
    pub posted_at: chrono::DateTime<chrono::Utc>,
    pub amount_idr: i64,
    pub allocated_idr: i64,
}

pub struct Allocation {
    pub debit_entry_id: Uuid,
    pub amount_idr: i64,
}

pub fn allocate_fifo(amount_idr: i64, debits: &mut [OpenDebit]) -> Option<Vec<Allocation>> {
    if amount_idr <= 0
        || debits.iter().any(|debit| {
            debit.amount_idr <= 0
                || debit.allocated_idr < 0
                || debit.allocated_idr > debit.amount_idr
        })
    {
        return None;
    }
    debits.sort_by_key(|debit| {
        (
            debit.effective_date.is_none(),
            debit.effective_date,
            debit.posted_at,
            debit.entry_id,
        )
    });
    let mut remaining = amount_idr;
    let mut allocations = Vec::new();
    for debit in debits {
        let open = debit.amount_idr - debit.allocated_idr;
        let allocated = open.min(remaining);
        if allocated > 0 {
            allocations.push(Allocation {
                debit_entry_id: debit.entry_id,
                amount_idr: allocated,
            });
            remaining -= allocated;
        }
        if remaining == 0 {
            break;
        }
    }
    Some(allocations)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fifo_splits_and_preserves_surplus() {
        let first = Uuid::from_u128(1);
        let second = Uuid::from_u128(2);
        let mut debits = vec![
            OpenDebit {
                entry_id: second,
                effective_date: NaiveDate::from_ymd_opt(2026, 1, 2),
                posted_at: chrono::DateTime::<chrono::Utc>::UNIX_EPOCH,
                amount_idr: 200000,
                allocated_idr: 0,
            },
            OpenDebit {
                entry_id: first,
                effective_date: NaiveDate::from_ymd_opt(2026, 1, 1),
                posted_at: chrono::DateTime::<chrono::Utc>::UNIX_EPOCH,
                amount_idr: 100000,
                allocated_idr: 0,
            },
        ];
        let result = allocate_fifo(150000, &mut debits).unwrap();
        assert_eq!(
            result
                .iter()
                .map(|a| (a.debit_entry_id, a.amount_idr))
                .collect::<Vec<_>>(),
            vec![(first, 100000), (second, 50000)]
        );
        assert_eq!(150000 - result.iter().map(|a| a.amount_idr).sum::<i64>(), 0);
    }
}
