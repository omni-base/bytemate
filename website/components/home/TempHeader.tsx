'use client'

import styles from './TempHeader.module.css';

export default function TempHeader() {
  return (
    <div className={styles.container}>
        <div className={styles.header}>
            <h1>ByteMate</h1>
            <p>Your friendly neighborhood discord bot</p>

        </div>
    </div>
  );
}